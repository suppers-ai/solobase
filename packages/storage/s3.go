package storage

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/aws-sdk-go-v2/service/s3/types"
)

// S3Provider implements storage using AWS S3 or S3-compatible services
type S3Provider struct {
	client        *s3.Client
	presignClient *s3.PresignClient
	region        string
	bucketPrefix  string
	baseURL       string
}

// NewS3Provider creates a new S3 storage provider
func NewS3Provider(cfg Config) (*S3Provider, error) {
	// Create AWS config
	awsCfg, err := createAWSConfig(cfg)
	if err != nil {
		return nil, fmt.Errorf("failed to create AWS config: %w", err)
	}

	// Create S3 client
	client := s3.NewFromConfig(awsCfg, func(opts *s3.Options) {
		if cfg.S3Endpoint != "" {
			opts.BaseEndpoint = aws.String(cfg.S3Endpoint)
		}
		if cfg.S3PathStyle {
			opts.UsePathStyle = true
		}
	})

	// Create presign client
	presignClient := s3.NewPresignClient(client)

	return &S3Provider{
		client:        client,
		presignClient: presignClient,
		region:        cfg.S3Region,
		bucketPrefix:  cfg.S3BucketPrefix,
		baseURL:       cfg.BaseURL,
	}, nil
}

func createAWSConfig(cfg Config) (aws.Config, error) {
	// Build config options
	opts := []func(*config.LoadOptions) error{
		config.WithRegion(cfg.S3Region),
	}

	// Add credentials if provided
	if cfg.S3AccessKeyID != "" && cfg.S3SecretAccessKey != "" {
		opts = append(opts, config.WithCredentialsProvider(
			credentials.NewStaticCredentialsProvider(
				cfg.S3AccessKeyID,
				cfg.S3SecretAccessKey,
				"",
			),
		))
	}

	// Load config
	awsCfg, err := config.LoadDefaultConfig(context.Background(), opts...)
	if err != nil {
		return aws.Config{}, err
	}

	return awsCfg, nil
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

	input := &s3.CreateBucketInput{
		Bucket: aws.String(bucketName),
	}

	// Set region-specific configuration
	if s.region != "" && s.region != "us-east-1" {
		input.CreateBucketConfiguration = &types.CreateBucketConfiguration{
			LocationConstraint: types.BucketLocationConstraint(s.region),
		}
	}

	// Create the bucket
	_, err := s.client.CreateBucket(ctx, input)
	if err != nil {
		return fmt.Errorf("failed to create bucket: %w", err)
	}

	// Configure public access if requested
	if opts.Public {
		// Remove public access block
		_, err = s.client.DeletePublicAccessBlock(ctx, &s3.DeletePublicAccessBlockInput{
			Bucket: aws.String(bucketName),
		})
		if err != nil {
			// Non-fatal: some S3-compatible services don't support this
		}

		// Set bucket policy for public read
		policy := fmt.Sprintf(`{
			"Version": "2012-10-17",
			"Statement": [{
				"Sid": "PublicReadGetObject",
				"Effect": "Allow",
				"Principal": "*",
				"Action": "s3:GetObject",
				"Resource": "arn:aws:s3:::%s/*"
			}]
		}`, bucketName)

		_, err = s.client.PutBucketPolicy(ctx, &s3.PutBucketPolicyInput{
			Bucket: aws.String(bucketName),
			Policy: aws.String(policy),
		})
		if err != nil {
			// Non-fatal: log but continue
		}
	}

	// Enable versioning if requested
	if opts.Versioning {
		_, err = s.client.PutBucketVersioning(ctx, &s3.PutBucketVersioningInput{
			Bucket: aws.String(bucketName),
			VersioningConfiguration: &types.VersioningConfiguration{
				Status: types.BucketVersioningStatusEnabled,
			},
		})
		if err != nil {
			// Non-fatal: log but continue
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
	_, err = s.client.DeleteBucket(ctx, &s3.DeleteBucketInput{
		Bucket: aws.String(bucketName),
	})
	if err != nil {
		return fmt.Errorf("failed to delete bucket: %w", err)
	}

	return nil
}

// BucketExists checks if a bucket exists
func (s *S3Provider) BucketExists(ctx context.Context, name string) (bool, error) {
	bucketName := s.getBucketName(name)

	_, err := s.client.HeadBucket(ctx, &s3.HeadBucketInput{
		Bucket: aws.String(bucketName),
	})
	if err != nil {
		// Check if it's a not found error
		if strings.Contains(err.Error(), "NotFound") || strings.Contains(err.Error(), "NoSuchBucket") {
			return false, nil
		}
		return false, err
	}

	return true, nil
}

// ListBuckets lists all S3 buckets
func (s *S3Provider) ListBuckets(ctx context.Context) ([]BucketInfo, error) {
	output, err := s.client.ListBuckets(ctx, &s3.ListBucketsInput{})
	if err != nil {
		return nil, fmt.Errorf("failed to list buckets: %w", err)
	}

	var buckets []BucketInfo
	for _, bucket := range output.Buckets {
		// Filter by prefix if configured
		if s.bucketPrefix != "" && !strings.HasPrefix(*bucket.Name, s.bucketPrefix) {
			continue
		}

		// Get bucket location
		location, _ := s.client.GetBucketLocation(ctx, &s3.GetBucketLocationInput{
			Bucket: bucket.Name,
		})

		region := "us-east-1"
		if location != nil && location.LocationConstraint != "" {
			region = string(location.LocationConstraint)
		}

		// Get bucket stats (simplified - in production, you'd want to paginate)
		objectCount, totalSize := s.getBucketStats(ctx, *bucket.Name)

		// Check if bucket is public
		public := s.isBucketPublic(ctx, *bucket.Name)

		// Remove prefix from display name
		displayName := *bucket.Name
		if s.bucketPrefix != "" {
			displayName = strings.TrimPrefix(displayName, s.bucketPrefix)
		}

		buckets = append(buckets, BucketInfo{
			Name:        displayName,
			CreatedAt:   *bucket.CreationDate,
			Public:      public,
			Region:      region,
			ObjectCount: objectCount,
			TotalSize:   totalSize,
		})
	}

	return buckets, nil
}

// PutObject uploads an object to S3
func (s *S3Provider) PutObject(ctx context.Context, bucket, key string, reader io.Reader, size int64, opts PutObjectOptions) error {
	bucketName := s.getBucketName(bucket)

	// Read all data into memory for S3 upload
	// For large files, you'd want to use multipart upload
	data, err := io.ReadAll(reader)
	if err != nil {
		return fmt.Errorf("failed to read data: %w", err)
	}

	input := &s3.PutObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
		Body:   bytes.NewReader(data),
	}

	// Set content type
	if opts.ContentType != "" {
		input.ContentType = aws.String(opts.ContentType)
	}

	// Set content encoding
	if opts.ContentEncoding != "" {
		input.ContentEncoding = aws.String(opts.ContentEncoding)
	}

	// Set cache control
	if opts.CacheControl != "" {
		input.CacheControl = aws.String(opts.CacheControl)
	}

	// Set metadata
	if len(opts.Metadata) > 0 {
		input.Metadata = opts.Metadata
	}

	// Set ACL
	if opts.Public {
		input.ACL = types.ObjectCannedACLPublicRead
	}

	_, err = s.client.PutObject(ctx, input)
	if err != nil {
		return fmt.Errorf("failed to upload object: %w", err)
	}

	return nil
}

// GetObject retrieves an object from S3
func (s *S3Provider) GetObject(ctx context.Context, bucket, key string) (io.ReadCloser, error) {
	bucketName := s.getBucketName(bucket)

	output, err := s.client.GetObject(ctx, &s3.GetObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
	})
	if err != nil {
		if strings.Contains(err.Error(), "NoSuchKey") {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to get object: %w", err)
	}

	return output.Body, nil
}

// GetObjectInfo retrieves information about an object
func (s *S3Provider) GetObjectInfo(ctx context.Context, bucket, key string) (*ObjectInfo, error) {
	bucketName := s.getBucketName(bucket)

	output, err := s.client.HeadObject(ctx, &s3.HeadObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
	})
	if err != nil {
		if strings.Contains(err.Error(), "NotFound") {
			return nil, fmt.Errorf("object not found")
		}
		return nil, fmt.Errorf("failed to get object info: %w", err)
	}

	info := &ObjectInfo{
		Key:         key,
		Size:        *output.ContentLength,
		ContentType: aws.ToString(output.ContentType),
	}

	if output.ETag != nil {
		info.ETag = strings.Trim(*output.ETag, "\"")
	}

	if output.LastModified != nil {
		info.LastModified = *output.LastModified
	}

	if output.Metadata != nil {
		info.Metadata = output.Metadata
	}

	return info, nil
}

// DeleteObject deletes an object from S3
func (s *S3Provider) DeleteObject(ctx context.Context, bucket, key string) error {
	bucketName := s.getBucketName(bucket)

	_, err := s.client.DeleteObject(ctx, &s3.DeleteObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
	})
	if err != nil {
		return fmt.Errorf("failed to delete object: %w", err)
	}

	return nil
}

// ListObjects lists objects in an S3 bucket
func (s *S3Provider) ListObjects(ctx context.Context, bucket, prefix string, opts ListObjectsOptions) ([]ObjectInfo, error) {
	bucketName := s.getBucketName(bucket)

	input := &s3.ListObjectsV2Input{
		Bucket: aws.String(bucketName),
	}

	if prefix != "" {
		input.Prefix = aws.String(prefix)
	}

	if opts.MaxKeys > 0 {
		input.MaxKeys = aws.Int32(int32(opts.MaxKeys))
	}

	if opts.Delimiter != "" {
		input.Delimiter = aws.String(opts.Delimiter)
	}

	if opts.Marker != "" {
		input.StartAfter = aws.String(opts.Marker)
	}

	var objects []ObjectInfo

	// Use paginator for complete results
	paginator := s3.NewListObjectsV2Paginator(s.client, input)
	for paginator.HasMorePages() {
		output, err := paginator.NextPage(ctx)
		if err != nil {
			return nil, fmt.Errorf("failed to list objects: %w", err)
		}

		// Add regular objects
		for _, obj := range output.Contents {
			objects = append(objects, ObjectInfo{
				Key:          aws.ToString(obj.Key),
				Size:         *obj.Size,
				ETag:         strings.Trim(aws.ToString(obj.ETag), "\""),
				LastModified: *obj.LastModified,
				IsDir:        false,
			})
		}

		// Add directories (common prefixes)
		for _, prefix := range output.CommonPrefixes {
			objects = append(objects, ObjectInfo{
				Key:   aws.ToString(prefix.Prefix),
				IsDir: true,
			})
		}

		// Check if we've reached the max keys limit
		if opts.MaxKeys > 0 && len(objects) >= opts.MaxKeys {
			break
		}
	}

	return objects, nil
}

// GeneratePresignedURL generates a presigned URL for temporary access
func (s *S3Provider) GeneratePresignedURL(ctx context.Context, bucket, key string, expires time.Duration) (string, error) {
	bucketName := s.getBucketName(bucket)

	request, err := s.presignClient.PresignGetObject(ctx, &s3.GetObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
	}, func(opts *s3.PresignOptions) {
		opts.Expires = expires
	})

	if err != nil {
		return "", fmt.Errorf("failed to generate presigned URL: %w", err)
	}

	return request.URL, nil
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
	paginator := s3.NewListObjectsV2Paginator(s.client, &s3.ListObjectsV2Input{
		Bucket: aws.String(bucket),
	})

	for paginator.HasMorePages() {
		output, err := paginator.NextPage(ctx)
		if err != nil {
			return err
		}

		if len(output.Contents) == 0 {
			continue
		}

		// Build delete objects
		var deleteObjects []types.ObjectIdentifier
		for _, obj := range output.Contents {
			deleteObjects = append(deleteObjects, types.ObjectIdentifier{
				Key: obj.Key,
			})
		}

		// Delete objects in batch
		_, err = s.client.DeleteObjects(ctx, &s3.DeleteObjectsInput{
			Bucket: aws.String(bucket),
			Delete: &types.Delete{
				Objects: deleteObjects,
				Quiet:   aws.Bool(true),
			},
		})
		if err != nil {
			return err
		}
	}

	return nil
}

func (s *S3Provider) getBucketStats(ctx context.Context, bucket string) (int64, int64) {
	var count int64
	var size int64

	paginator := s3.NewListObjectsV2Paginator(s.client, &s3.ListObjectsV2Input{
		Bucket: aws.String(bucket),
	})

	for paginator.HasMorePages() {
		output, err := paginator.NextPage(ctx)
		if err != nil {
			break
		}

		for _, obj := range output.Contents {
			count++
			size += *obj.Size
		}
	}

	return count, size
}

func (s *S3Provider) isBucketPublic(ctx context.Context, bucket string) bool {
	// Check bucket policy
	policy, err := s.client.GetBucketPolicy(ctx, &s3.GetBucketPolicyInput{
		Bucket: aws.String(bucket),
	})
	if err == nil && policy.Policy != nil {
		// Simple check for public read policy
		if strings.Contains(*policy.Policy, "\"Principal\":\"*\"") ||
			strings.Contains(*policy.Policy, "\"Principal\":{\"AWS\":\"*\"}") {
			return true
		}
	}

	// Check bucket ACL
	acl, err := s.client.GetBucketAcl(ctx, &s3.GetBucketAclInput{
		Bucket: aws.String(bucket),
	})
	if err == nil && acl.Grants != nil {
		for _, grant := range acl.Grants {
			if grant.Grantee != nil && grant.Grantee.Type == types.TypeGroup {
				if grant.Grantee.URI != nil && strings.Contains(*grant.Grantee.URI, "AllUsers") {
					return true
				}
			}
		}
	}

	return false
}
