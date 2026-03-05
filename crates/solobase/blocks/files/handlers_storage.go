package files

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"log"
	"mime"
	"mime/multipart"
	"strconv"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/constants"
	"github.com/suppers-ai/solobase/core/uuid"
	wafer "github.com/wafer-run/wafer-go"
)

// ========== Bucket Handlers ==========

func (b *FilesBlock) handleGetBuckets(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.storageService == nil {
		return wafer.JSONRespond(msg, 200, []any{})
	}
	buckets, err := b.storageService.GetBuckets()
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to fetch buckets")
	}
	return wafer.JSONRespond(msg, 200, buckets)
}

func (b *FilesBlock) handleCreateBucket(_ wafer.Context, msg *wafer.Message) wafer.Result {
	var request struct {
		Name   string `json:"name"`
		Public bool   `json:"public"`
	}
	if err := msg.Decode(&request); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}
	if request.Name == "" {
		return wafer.Error(msg, 400, "validation_error", "Bucket name is required")
	}

	if err := b.storageService.CreateBucket(request.Name, request.Public); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 201, map[string]any{
		"message": "Bucket created successfully",
		"name":    request.Name,
	})
}

func (b *FilesBlock) handleDeleteBucket(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")

	if err := b.storageService.DeleteBucket(bucket); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, map[string]string{"message": "Bucket deleted successfully"})
}

// ========== Object Handlers ==========

func (b *FilesBlock) handleGetBucketObjects(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	userID := msg.UserID()

	parentFolderIDStr := msg.Query("parent_folder_id")
	var parentFolderID *string
	if parentFolderIDStr != "" {
		parentFolderID = &parentFolderIDStr
	}

	filterByUser := ""
	if IsInternalBucket(bucket) {
		bucket = constants.InternalStorageBucket
		filterByUser = userID
	}

	objects, err := b.storageService.GetObjects(bucket, filterByUser, parentFolderID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to fetch objects")
	}
	return wafer.JSONRespond(msg, 200, objects)
}

func (b *FilesBlock) handleUploadFile(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	userID := msg.UserID()

	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	contentType := msg.ContentType()
	_, params, err := mime.ParseMediaType(contentType)
	if err != nil {
		return wafer.Error(msg, 400, "invalid_content", "Invalid content type")
	}
	boundary := params["boundary"]
	if boundary == "" {
		return wafer.Error(msg, 400, "invalid_content", "Missing multipart boundary")
	}

	reader := multipart.NewReader(bytes.NewReader(msg.Data), boundary)
	var fileContent []byte
	var filename string
	var fileContentType string
	var parentFolderID string

	for {
		part, err := reader.NextPart()
		if err == io.EOF {
			break
		}
		if err != nil {
			return wafer.Error(msg, 400, "parse_error", "Failed to parse multipart form")
		}

		fieldName := part.FormName()
		if fieldName == "file" {
			filename = part.FileName()
			fileContentType = part.Header.Get("Content-Type")
			fileContent, err = io.ReadAll(part)
			if err != nil {
				return wafer.Error(msg, 500, "read_error", "Failed to read file")
			}
		} else if fieldName == "parent_folder_id" {
			data, _ := io.ReadAll(part)
			parentFolderID = string(data)
		}
		part.Close()
	}

	if len(fileContent) == 0 || filename == "" {
		return wafer.Error(msg, 400, "missing_file", "Failed to get file")
	}

	if fileContentType == "" {
		fileContentType = "application/octet-stream"
	}

	if IsInternalBucket(bucket) {
		bucket = constants.InternalStorageBucket
	}

	fileSize := int64(len(fileContent))

	// Check quota before upload
	if err := b.beforeUpload(context.Background(), userID, bucket, filename, fileSize); err != nil {
		return wafer.Error(msg, 507, "quota_exceeded", err.Error())
	}

	var parentFolderPtr *string
	if parentFolderID != "" {
		parentFolderPtr = &parentFolderID
	}

	object, err := b.storageService.UploadFile(bucket, filename, userID, bytes.NewReader(fileContent), fileSize, fileContentType, parentFolderPtr)
	if err != nil {
		return wafer.Error(msg, 500, "upload_error", "Failed to upload file: "+err.Error())
	}

	// Update usage after upload
	objectID := ""
	if objMap, ok := object.(map[string]any); ok {
		if id, ok := objMap["id"].(string); ok {
			objectID = id
		}
	}
	go b.afterUpload(context.Background(), userID, bucket, objectID, filename, fileSize)

	return wafer.JSONRespond(msg, 201, object)
}

func (b *FilesBlock) handleDeleteObject(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	objectID := msg.Var("id")
	userID := msg.UserID()

	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	var err error
	if IsInternalBucket(bucket) {
		objectInfo, getErr := b.storageService.GetObjectInfo(constants.InternalStorageBucket, objectID)
		if getErr != nil {
			return wafer.Error(msg, 404, "not_found", "Object not found")
		}

		ownershipErr := CheckStorageOwnership(userID, &StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, b.storageService.GetAppID())
		if ownershipErr != nil {
			return b.ownershipError(msg, ownershipErr)
		}

		err = b.storageService.DeleteObject(constants.InternalStorageBucket, objectID)
	} else {
		err = b.storageService.DeleteObject(bucket, objectID)
	}

	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to delete object: "+err.Error())
	}
	return wafer.JSONRespond(msg, 200, map[string]string{"message": "Object deleted successfully"})
}

func (b *FilesBlock) handleDownloadObject(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	objectID := msg.Var("id")
	userID := msg.UserID()

	actualBucket := bucket
	if IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		if userID == "" {
			return wafer.Error(msg, 401, "unauthorized", "Authentication required")
		}

		objectInfo, err := b.storageService.GetObjectInfo(actualBucket, objectID)
		if err != nil {
			return wafer.Error(msg, 404, "not_found", "Object not found")
		}

		ownershipErr := CheckStorageOwnership(userID, &StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, b.storageService.GetAppID())
		if ownershipErr != nil {
			return b.ownershipError(msg, ownershipErr)
		}
	}

	// Check share permissions before download
	if err := b.beforeDownload(context.Background(), userID, bucket, objectID); err != nil {
		return wafer.Error(msg, 403, "forbidden", err.Error())
	}

	reader, filename, contentType, err := b.storageService.GetObject(actualBucket, objectID)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", "Object not found")
	}
	defer reader.Close()

	data, err := io.ReadAll(reader)
	if err != nil {
		return wafer.Error(msg, 500, "read_error", "Failed to read file")
	}

	// Update bandwidth usage after download
	if userID != "" {
		go b.afterDownload(context.Background(), userID, bucket, objectID, int64(len(data)))
	}

	return wafer.NewResponse(msg, 200).
		SetHeader("Content-Disposition", `attachment; filename="`+filename+`"`).
		Body(data, contentType)
}

func (b *FilesBlock) handleGetObject(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	objectID := msg.Var("id")
	userID := msg.UserID()

	actualBucket := bucket
	if IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		if userID == "" {
			return wafer.Error(msg, 401, "unauthorized", "Authentication required")
		}
	}

	objectInfo, err := b.storageService.GetObjectInfo(actualBucket, objectID)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", "Object not found")
	}

	if IsInternalBucket(bucket) {
		ownershipErr := CheckStorageOwnership(userID, &StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, b.storageService.GetAppID())
		if ownershipErr != nil {
			return b.ownershipError(msg, ownershipErr)
		}
	}

	return wafer.JSONRespond(msg, 200, objectInfo)
}

func (b *FilesBlock) handleRenameObject(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	objectID := msg.Var("id")
	userID := msg.UserID()

	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	var request struct {
		Name string `json:"name"`
	}
	if err := msg.Decode(&request); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}
	if request.Name == "" {
		return wafer.Error(msg, 400, "validation_error", "New name is required")
	}

	actualBucket := bucket
	if IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		objectInfo, err := b.storageService.GetObjectInfo(actualBucket, objectID)
		if err != nil {
			return wafer.Error(msg, 404, "not_found", "Object not found")
		}

		ownershipErr := CheckStorageOwnership(userID, &StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, b.storageService.GetAppID())
		if ownershipErr != nil {
			return b.ownershipError(msg, ownershipErr)
		}
	}

	if err := b.storageService.RenameObject(actualBucket, objectID, request.Name); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, map[string]any{"message": "Object renamed successfully"})
}

func (b *FilesBlock) handleCreateFolder(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	userID := msg.UserID()

	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	var request struct {
		Name           string  `json:"name"`
		Path           string  `json:"path"`
		ParentFolderID *string `json:"parentFolderId"`
	}
	if err := msg.Decode(&request); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}
	if request.Name == "" {
		return wafer.Error(msg, 400, "validation_error", "Folder name is required")
	}

	actualBucket := bucket
	if IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket
	}

	folderID, err := b.storageService.CreateFolderWithParent(actualBucket, request.Name, userID, request.ParentFolderID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to create folder: "+err.Error())
	}

	folder, err := b.storageService.GetObjectInfo(actualBucket, folderID)
	if err != nil {
		return wafer.JSONRespond(msg, 201, map[string]any{
			"id":      folderID,
			"name":    request.Name,
			"message": "Folder created successfully",
		})
	}

	return wafer.JSONRespond(msg, 201, map[string]any{
		"id":               folder.ID,
		"bucket_name":      folder.BucketName,
		"object_name":      folder.ObjectName,
		"parent_folder_id": folder.ParentFolderID,
		"size":             folder.Size,
		"content_type":     folder.ContentType,
		"checksum":         folder.Checksum,
		"metadata":         folder.Metadata,
		"created_at":       folder.CreatedAt,
		"updated_at":       folder.UpdatedAt,
		"last_viewed":      folder.LastViewed,
		"user_id":          folder.UserID,
		"app_id":           folder.AppID,
		"message":          "Folder created successfully",
	})
}

// ========== Presigned URL Handlers ==========

func (b *FilesBlock) handleGenerateDownloadURL(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	objectID := msg.Var("id")
	userID := msg.UserID()

	object, err := b.storageService.GetObjectInfo(bucket, objectID)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", "Object not found")
	}

	provider := b.storageService.GetProviderType()

	tokenStr := uuid.New().String()
	tokenID := uuid.New().String()
	fileSize := object.Size
	expiresAt := apptime.NowTime().Add(apptime.Hour)

	downloadToken := &DownloadToken{
		ID:             tokenID,
		Token:          tokenStr,
		FileID:         objectID,
		Bucket:         bucket,
		ParentFolderID: object.ParentFolderID,
		ObjectName:     object.ObjectName,
		UserID:         nilIfEmpty(userID),
		FileSize:       &fileSize,
		ExpiresAt:      apptime.NewNullTime(expiresAt),
		CreatedAt:      apptime.NowTime(),
	}

	var response map[string]any

	if provider == "s3" {
		url, err := b.storageService.GeneratePresignedDownloadURL(bucket, object.ObjectName, constants.DefaultURLExpiry)
		if err != nil {
			return wafer.Error(msg, 500, "internal_error", "Failed to generate download URL")
		}

		if err := b.storageService.CreateDownloadToken(context.Background(), downloadToken); err != nil {
			log.Printf("Failed to create download token: %v", err)
		}

		response = map[string]any{
			"url":          url,
			"type":         "presigned",
			"expires_in":   constants.DefaultURLExpiry,
			"callback_url": fmt.Sprintf("/api/storage/download-callback/%s", tokenStr),
		}
	} else {
		if err := b.storageService.CreateDownloadToken(context.Background(), downloadToken); err != nil {
			return wafer.Error(msg, 500, "internal_error", "Failed to create download token")
		}

		response = map[string]any{
			"url":        fmt.Sprintf("/api/storage/direct/%s", tokenStr),
			"type":       "token",
			"expires_in": constants.DefaultURLExpiry,
		}
	}

	return wafer.JSONRespond(msg, 200, response)
}

func (b *FilesBlock) handleGenerateUploadURL(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	userID := msg.UserID()

	var request struct {
		Filename       string  `json:"filename"`
		ParentFolderID *string `json:"parentFolderId,omitempty"`
		ContentType    string  `json:"contentType"`
		MaxSize        int64   `json:"maxSize"`
	}
	if err := msg.Decode(&request); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}
	if request.Filename == "" {
		return wafer.Error(msg, 400, "validation_error", "Filename is required")
	}
	if request.ContentType == "" {
		request.ContentType = "application/octet-stream"
	}
	if request.MaxSize == 0 {
		request.MaxSize = constants.DefaultMaxFileSize
	}

	objectKey := request.Filename

	// Check storage quota before generating URL
	if userID != "" {
		if err := b.beforeUpload(context.Background(), userID, bucket, request.Filename, request.MaxSize); err != nil {
			return wafer.Error(msg, 507, "quota_exceeded", err.Error())
		}
	}

	provider := b.storageService.GetProviderType()

	tokenStr := uuid.New().String()
	tokenID := uuid.New().String()
	expiresAt := apptime.NowTime().Add(apptime.Hour)

	uploadToken := &UploadToken{
		ID:             tokenID,
		Token:          tokenStr,
		Bucket:         bucket,
		ParentFolderID: request.ParentFolderID,
		ObjectName:     request.Filename,
		UserID:         nilIfEmpty(userID),
		MaxSize:        &request.MaxSize,
		ContentType:    &request.ContentType,
		ExpiresAt:      apptime.NewNullTime(expiresAt),
		CreatedAt:      apptime.NowTime(),
	}

	var response map[string]any

	if provider == "s3" {
		url, err := b.storageService.GeneratePresignedUploadURL(bucket, objectKey, request.ContentType, constants.DefaultURLExpiry)
		if err != nil {
			return wafer.Error(msg, 500, "internal_error", "Failed to generate upload URL")
		}

		if err := b.storageService.CreateUploadToken(context.Background(), uploadToken); err != nil {
			log.Printf("Failed to create upload token: %v", err)
		}

		response = map[string]any{
			"url":          url,
			"type":         "presigned",
			"expires_in":   constants.DefaultURLExpiry,
			"callback_url": fmt.Sprintf("/api/storage/upload-callback/%s", tokenStr),
		}
	} else {
		if err := b.storageService.CreateUploadToken(context.Background(), uploadToken); err != nil {
			return wafer.Error(msg, 500, "internal_error", "Failed to create upload token")
		}

		response = map[string]any{
			"url":        fmt.Sprintf("/api/storage/direct-upload/%s", tokenStr),
			"type":       "token",
			"expires_in": constants.DefaultURLExpiry,
		}
	}

	return wafer.JSONRespond(msg, 200, response)
}

// ========== Direct Download ==========

func (b *FilesBlock) handleDirectDownload(_ wafer.Context, msg *wafer.Message) wafer.Result {
	tokenStr := msg.Var("token")

	token, err := b.storageService.GetDownloadTokenByToken(context.Background(), tokenStr)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", "Invalid or expired token")
	}

	if token.ExpiresAt.Valid && apptime.NowTime().After(token.ExpiresAt.Time) {
		return wafer.Error(msg, 401, "expired", "Token has expired")
	}

	reader, filename, contentType, err := b.storageService.GetObjectByKey(token.Bucket, token.ObjectName)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", "File not found")
	}
	defer reader.Close()

	data, err := io.ReadAll(reader)
	if err != nil {
		return wafer.Error(msg, 500, "read_error", "Failed to read file")
	}

	// Update token progress
	_ = b.storageService.UpdateDownloadTokenProgress(context.Background(), token.ID, int64(len(data)))
	_ = b.storageService.CompleteDownloadToken(context.Background(), token.ID)

	return wafer.NewResponse(msg, 200).
		SetHeader("Content-Disposition", `attachment; filename="`+filename+`"`).
		Body(data, contentType)
}

// ========== Search & Utilities ==========

func (b *FilesBlock) handleSearchObjects(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	query := msg.Query("q")
	if query == "" {
		return wafer.JSONRespond(msg, 200, map[string]any{"items": []any{}})
	}

	appID := msg.Query("app_id")
	if appID == "" {
		appID = "default"
	}

	objects, err := b.storageService.SearchStorageObjects(userID, appID, query, 50)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to search items")
	}

	result := make([]map[string]any, 0, len(objects))
	for _, obj := range objects {
		result = append(result, map[string]any{
			"id":               obj.ID,
			"bucket_name":      obj.BucketName,
			"object_name":      obj.ObjectName,
			"parent_folder_id": obj.ParentFolderID,
			"size":             obj.Size,
			"content_type":     obj.ContentType,
			"checksum":         obj.Checksum,
			"metadata":         obj.Metadata,
			"created_at":       obj.CreatedAt,
			"updated_at":       obj.UpdatedAt,
			"last_viewed":      obj.LastViewed,
			"user_id":          obj.UserID,
			"app_id":           obj.AppID,
		})
	}

	return wafer.JSONRespond(msg, 200, map[string]any{"items": result})
}

func (b *FilesBlock) handleGetRecentlyViewed(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	limit := 30
	if limitStr := msg.Query("limit"); limitStr != "" {
		if l, err := strconv.Atoi(limitStr); err == nil && l > 0 && l <= 100 {
			limit = l
		}
	}

	objects, err := b.storageService.GetRecentlyViewed(userID, limit)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to get recently viewed items")
	}

	response := make([]map[string]any, 0, len(objects))
	for _, obj := range objects {
		response = append(response, map[string]any{
			"id":               obj.ID,
			"bucket_name":      obj.BucketName,
			"object_name":      obj.ObjectName,
			"parent_folder_id": obj.ParentFolderID,
			"size":             obj.Size,
			"content_type":     obj.ContentType,
			"checksum":         obj.Checksum,
			"metadata":         obj.Metadata,
			"created_at":       obj.CreatedAt,
			"updated_at":       obj.UpdatedAt,
			"last_viewed":      obj.LastViewed,
			"user_id":          obj.UserID,
			"app_id":           obj.AppID,
		})
	}

	return wafer.JSONRespond(msg, 200, response)
}

func (b *FilesBlock) handleUpdateLastViewed(_ wafer.Context, msg *wafer.Message) wafer.Result {
	itemID := msg.Var("id")
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	if err := b.storageService.UpdateLastViewed(itemID, userID); err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to update last viewed")
	}
	return wafer.JSONRespond(msg, 200, map[string]any{
		"message":     "Last viewed updated successfully",
		"last_viewed": apptime.NowTime(),
	})
}

func (b *FilesBlock) handleGetQuota(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	storageUsed, err := b.storageService.GetUserStorageUsed(userID)
	if err != nil {
		log.Printf("Failed to get user storage: %v", err)
		storageUsed = 0
	}

	maxStorage := int64(constants.DefaultMaxStorageBytes)
	maxBandwidth := int64(constants.DefaultMaxBandwidthBytes)

	percentage := float64(0)
	if maxStorage > 0 {
		percentage = (float64(storageUsed) / float64(maxStorage)) * 100
	}

	return wafer.JSONRespond(msg, 200, map[string]any{
		"used":            storageUsed,
		"total":           maxStorage,
		"percentage":      percentage,
		"storage_used":    storageUsed,
		"storage_limit":   maxStorage,
		"bandwidth_used":  int64(0),
		"bandwidth_limit": maxBandwidth,
	})
}

func (b *FilesBlock) handleGetStats(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	stats, err := b.storageService.GetStorageStats(userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to get storage statistics")
	}

	return wafer.JSONRespond(msg, 200, map[string]any{
		"totalSize":     stats["totalSize"],
		"fileCount":     stats["fileCount"],
		"folderCount":   stats["folderCount"],
		"sharedCount":   stats["sharedCount"],
		"recentUploads": stats["recentUploads"],
		"trashedCount":  0,
		"totalFiles":    stats["fileCount"],
		"totalFolders":  stats["folderCount"],
	})
}

// ========== Admin Handlers ==========

func (b *FilesBlock) handleGetAdminStats(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if !msg.IsAdmin() {
		return wafer.Error(msg, 403, "forbidden", "Admin access required")
	}

	stats, err := b.storageService.GetAllUsersStorageStats()
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to get storage statistics")
	}
	return wafer.JSONRespond(msg, 200, stats)
}

// ========== Object Metadata ==========

func (b *FilesBlock) handleUpdateObjectMetadata(_ wafer.Context, msg *wafer.Message) wafer.Result {
	bucket := msg.Var("bucket")
	objectID := msg.Var("id")
	userID := msg.UserID()

	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Authentication required")
	}

	var request struct {
		Metadata string `json:"metadata"`
	}
	if err := msg.Decode(&request); err != nil {
		return wafer.Error(msg, 400, "invalid_body", "Invalid request body")
	}

	actualBucket := bucket
	if IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		objectInfo, err := b.storageService.GetObjectInfo(actualBucket, objectID)
		if err != nil {
			return wafer.Error(msg, 404, "not_found", "Object not found")
		}

		ownershipErr := CheckStorageOwnership(userID, &StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, b.storageService.GetAppID())
		if ownershipErr != nil {
			return b.ownershipError(msg, ownershipErr)
		}
	}

	if err := b.storageService.UpdateObjectMetadata(actualBucket, objectID, request.Metadata); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	objectInfo, err := b.storageService.GetObjectInfo(actualBucket, objectID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to retrieve updated object")
	}
	return wafer.JSONRespond(msg, 200, objectInfo)
}
