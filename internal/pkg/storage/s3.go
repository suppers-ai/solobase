//go:build !wasm

package storage

import (
	"context"
	"fmt"
	"io"
	"strconv"
	"strings"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	"github.com/rhnvrm/simples3"
)

// S3Provider implements storage using AWS S3 or S3-compatible services
// Uses simples3 library which works in both standard and WASM builds
type S3Provider struct {
	client       *simples3.S3
	region       string
	bucketPrefix string
	baseURL      string
}

// NewS3Provider creates a new S3 storage provider
func NewS3Provider(cfg Config) (*S3Provider, error) {
	// Create simples3 client
	client := simples3.New(cfg.S3Region, cfg.S3AccessKeyID, cfg.S3SecretAccessKey)

	// Set custom endpoint if provided (for MinIO, R2, DigitalOcean Spaces, etc.)
	if cfg.S3Endpoint != "" {
		client.SetEndpoint(cfg.S3Endpoint)
	}

	return &S3Provider{
		client:       client,
		region:       cfg.S3Region,
		bucketPrefix: cfg.S3BucketPrefix,
		baseURL:      cfg.BaseURL,
	}, nil
}

// Name returns the provider name
func (s *S3Provider) Name() string {
	return "Amazon S3"
}

// Type returns the provider type
func (s *S3Provider) Type() ProviderType {
	return ProviderS3
}

// CreateBucket creates a new S3 bucket
func (s *S3Provider) CreateBucket(ctx context.Context, name string, opts CreateBucketOptions) error {
	bucketName := s.getBucketName(name)

	_, err := s.client.CreateBucket(simples3.CreateBucketInput{
		Bucket: bucketName,
		Region: s.region,
	})
	if err != nil {
		return fmt.Errorf("failed to create bucket: %w", err)
	}

	// Enable versioning if requested
	if opts.Versioning {
		err = s.client.PutBucketVersioning(simples3.PutBucketVersioningInput{
			Bucket: bucketName,
			Status: "Enabled",
		})
		if err != nil {
			// Non-fatal: log but continue
		}
	}

	// Set public ACL if requested
	if opts.Public {
		err = s.client.PutBucketAcl(simples3.PutBucketAclInput{
			Bucket:    bucketName,
			CannedACL: "public-read",
		})
		if err != nil {
			// Non-fatal: some S3-compatible services don't support this
		}
	}

	return nil
}

// DeleteBucket deletes an S3 bucket
func (s *S3Provider) DeleteBucket(ctx context.Context, name string) error {
	bucketName := s.getBucketName(name)

	// First, delete all objects in the bucket
	err := s.emptyBucket(ctx, bucketName)
	if err != nil {
		return fmt.Errorf("failed to empty bucket: %w", err)
	}

	// Delete the bucket
	err = s.client.DeleteBucket(simples3.DeleteBucketInput{
		Bucket: bucketName,
	})
	if err != nil {
		return fmt.Errorf("failed to delete bucket: %w", err)
	}

	return nil
}

// BucketExists checks if a bucket exists
func (s *S3Provider) BucketExists(ctx context.Context, name string) (bool, error) {
	bucketName := s.getBucketName(name)

	// List buckets and check if ours exists
	result, err := s.client.ListBuckets(simples3.ListBucketsInput{})
	if err != nil {
		return false, err
	}

	for _, bucket := range result.Buckets {
		if bucket.Name == bucketName {
			return true, nil
		}
	}

	return false, nil
}

// ListBuckets lists all S3 buckets
func (s *S3Provider) ListBuckets(ctx context.Context) ([]BucketInfo, error) {
	result, err := s.client.ListBuckets(simples3.ListBucketsInput{})
	if err != nil {
		return nil, fmt.Errorf("failed to list buckets: %w", err)
	}

	var buckets []BucketInfo
	for _, bucket := range result.Buckets {
		// Filter by prefix if configured
		if s.bucketPrefix != "" && !strings.HasPrefix(bucket.Name, s.bucketPrefix) {
			continue
		}

		// Get bucket stats
		objectCount, totalSize := s.getBucketStats(ctx, bucket.Name)

		// Remove prefix from display name
		displayName := bucket.Name
		if s.bucketPrefix != "" {
			displayName = strings.TrimPrefix(displayName, s.bucketPrefix)
		}

		buckets = append(buckets, BucketInfo{
			Name:        displayName,
			CreatedAt:   bucket.CreationDate,
			Region:      s.region,
			ObjectCount: objectCount,
			TotalSize:   totalSize,
		})
	}

	return buckets, nil
}

// PutObject uploads an object to S3
func (s *S3Provider) PutObject(ctx context.Context, bucket, key string, reader io.Reader, size int64, opts PutObjectOptions) error {
	bucketName := s.getBucketName(bucket)

	// Use multipart upload for all files since it accepts io.Reader
	// (regular FilePut requires io.ReadSeeker which we don't have)
	input := simples3.MultipartUploadInput{
		Bucket:      bucketName,
		ObjectKey:   key,
		ContentType: opts.ContentType,
		Body:        reader,
		PartSize:    5 * 1024 * 1024, // 5MB parts (minimum)
		Concurrency: 1,
	}

	// Set metadata
	if len(opts.Metadata) > 0 {
		input.CustomMetadata = opts.Metadata
	}

	_, err := s.client.FileUploadMultipart(input)
	if err != nil {
		return fmt.Errorf("failed to upload object: %w", err)
	}

	return nil
}

// GetObject retrieves an object from S3
func (s *S3Provider) GetObject(ctx context.Context, bucket, key string) (io.ReadCloser, error) {
	bucketName := s.getBucketName(bucket)

	body, err := s.client.FileDownload(simples3.DownloadInput{
		Bucket:    bucketName,
		ObjectKey: key,
	})
	if err != nil {
		if strings.Contains(err.Error(), "NoSuchKey") || strings.Contains(err.Error(), "404") {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to get object: %w", err)
	}

	return body, nil
}

// GetObjectInfo retrieves information about an object
func (s *S3Provider) GetObjectInfo(ctx context.Context, bucket, key string) (*ObjectInfo, error) {
	bucketName := s.getBucketName(bucket)

	details, err := s.client.FileDetails(simples3.DetailsInput{
		Bucket:    bucketName,
		ObjectKey: key,
	})
	if err != nil {
		if strings.Contains(err.Error(), "NotFound") || strings.Contains(err.Error(), "404") {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to get object info: %w", err)
	}

	lastModified, _ := apptime.ParseWithLayout(apptime.RFC1123, details.LastModified)
	contentLength, _ := strconv.ParseInt(details.ContentLength, 10, 64)

	info := &ObjectInfo{
		Key:          key,
		Size:         contentLength,
		ContentType:  details.ContentType,
		ETag:         strings.Trim(details.Etag, "\""),
		LastModified: lastModified,
	}

	if details.AmzMeta != nil {
		info.Metadata = details.AmzMeta
	}

	return info, nil
}

// DeleteObject deletes an object from S3
func (s *S3Provider) DeleteObject(ctx context.Context, bucket, key string) error {
	bucketName := s.getBucketName(bucket)

	err := s.client.FileDelete(simples3.DeleteInput{
		Bucket:    bucketName,
		ObjectKey: key,
	})
	if err != nil {
		return fmt.Errorf("failed to delete object: %w", err)
	}

	return nil
}

// ListObjects lists objects in an S3 bucket
func (s *S3Provider) ListObjects(ctx context.Context, bucket, prefix string, opts ListObjectsOptions) ([]ObjectInfo, error) {
	bucketName := s.getBucketName(bucket)

	input := simples3.ListInput{
		Bucket: bucketName,
	}

	if prefix != "" {
		input.Prefix = prefix
	}

	if opts.MaxKeys > 0 {
		input.MaxKeys = int64(opts.MaxKeys)
	}

	if opts.Delimiter != "" {
		input.Delimiter = opts.Delimiter
	}

	if opts.Marker != "" {
		input.ContinuationToken = opts.Marker
	}

	var objects []ObjectInfo

	// Paginate through results
	for {
		result, err := s.client.List(input)
		if err != nil {
			return nil, fmt.Errorf("failed to list objects: %w", err)
		}

		// Add regular objects
		for _, obj := range result.Objects {
			lastModified, _ := apptime.ParseWithLayout(apptime.TimeFormat, obj.LastModified)
			objects = append(objects, ObjectInfo{
				Key:          obj.Key,
				Size:         obj.Size,
				ETag:         strings.Trim(obj.ETag, "\""),
				LastModified: lastModified,
				IsDir:        false,
			})
		}

		// Add directories (common prefixes)
		for _, prefix := range result.CommonPrefixes {
			objects = append(objects, ObjectInfo{
				Key:   prefix,
				IsDir: true,
			})
		}

		// Check if we've reached the max keys limit
		if opts.MaxKeys > 0 && len(objects) >= opts.MaxKeys {
			break
		}

		// Check for more pages
		if !result.IsTruncated || result.NextContinuationToken == "" {
			break
		}

		input.ContinuationToken = result.NextContinuationToken
	}

	return objects, nil
}

// GeneratePresignedURL generates a presigned URL for temporary access
func (s *S3Provider) GeneratePresignedURL(ctx context.Context, bucket, key string, expires apptime.Duration) (string, error) {
	bucketName := s.getBucketName(bucket)

	url := s.client.GeneratePresignedURL(simples3.PresignedInput{
		Bucket:        bucketName,
		ObjectKey:     key,
		Method:        "GET",
		ExpirySeconds: int(expires.Seconds()),
	})

	return url, nil
}

// Helper functions

func (s *S3Provider) getBucketName(name string) string {
	if s.bucketPrefix != "" {
		return s.bucketPrefix + name
	}
	return name
}

func (s *S3Provider) emptyBucket(ctx context.Context, bucket string) error {
	// List all objects
	input := simples3.ListInput{
		Bucket: bucket,
	}

	for {
		result, err := s.client.List(input)
		if err != nil {
			return err
		}

		if len(result.Objects) == 0 {
			break
		}

		// Collect keys for batch delete
		var keys []string
		for _, obj := range result.Objects {
			keys = append(keys, obj.Key)
		}

		// Delete objects in batch
		_, err = s.client.DeleteObjects(simples3.DeleteObjectsInput{
			Bucket:  bucket,
			Objects: keys,
			Quiet:   true,
		})
		if err != nil {
			return err
		}

		// Check for more pages
		if !result.IsTruncated || result.NextContinuationToken == "" {
			break
		}

		input.ContinuationToken = result.NextContinuationToken
	}

	return nil
}

func (s *S3Provider) getBucketStats(ctx context.Context, bucket string) (int64, int64) {
	var count int64
	var size int64

	input := simples3.ListInput{
		Bucket: bucket,
	}

	for {
		result, err := s.client.List(input)
		if err != nil {
			break
		}

		for _, obj := range result.Objects {
			count++
			size += obj.Size
		}

		if !result.IsTruncated || result.NextContinuationToken == "" {
			break
		}

		input.ContinuationToken = result.NextContinuationToken
	}

	return count, size
}
