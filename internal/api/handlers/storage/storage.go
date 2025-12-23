package storage

import (
	"bytes"
	"context"
	"database/sql"
	"fmt"
	"io"
	"log"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/utils"
)

// StorageHandlers contains all storage-related handlers with hook support
type StorageHandlers struct {
	storageService *services.StorageService
	sqlDB          *sql.DB
	queries        *db.Queries
	hookRegistry   *core.ExtensionRegistry
}

// NewStorageHandlers creates new storage handlers with hook support
func NewStorageHandlers(storageService *services.StorageService, sqlDB *sql.DB, hookRegistry *core.ExtensionRegistry) *StorageHandlers {
	h := &StorageHandlers{
		storageService: storageService,
		sqlDB:          sqlDB,
		hookRegistry:   hookRegistry,
	}
	// Only create queries if sqlDB is available (not in WASM mode)
	if sqlDB != nil {
		h.queries = db.New(sqlDB)
	}
	return h
}

// HandleGetStorageBuckets handles bucket listing
func (h *StorageHandlers) HandleGetStorageBuckets(w http.ResponseWriter, r *http.Request) {
	fmt.Println("DEBUG: HandleGetStorageBuckets called")
	if h.storageService == nil {
		fmt.Println("DEBUG: storageService is nil")
		utils.JSONResponse(w, http.StatusOK, []interface{}{})
		return
	}
	fmt.Println("DEBUG: calling GetBuckets")
	buckets, err := h.storageService.GetBuckets()
	fmt.Printf("DEBUG: GetBuckets returned, err=%v\n", err)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch buckets")
		return
	}

	fmt.Printf("DEBUG: returning %d buckets\n", len(buckets))
	utils.JSONResponse(w, http.StatusOK, buckets)
}

// HandleGetBucketObjects handles object listing in a bucket
func (h *StorageHandlers) HandleGetBucketObjects(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]

	userID := utils.GetUserIDFromRequest(r)

	// Get parent folder ID from query
	parentFolderIDStr := r.URL.Query().Get("parent_folder_id")
	var parentFolderID *string
	if parentFolderIDStr != "" {
		parentFolderID = &parentFolderIDStr
	}

	// For user files, ensure we have a user ID
	filterByUser := ""
	if utils.IsInternalBucket(bucket) {
		bucket = constants.InternalStorageBucket
		// For int_storage, only filter by user if we have a userID
		filterByUser = userID
	}

	// Get objects - only filter by userID for int_storage when user is authenticated
	objects, err := h.storageService.GetObjects(bucket, filterByUser, parentFolderID)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch objects")
		return
	}

	utils.JSONResponse(w, http.StatusOK, objects)
}

// HandleCreateBucket handles bucket creation
func (h *StorageHandlers) HandleCreateBucket(w http.ResponseWriter, r *http.Request) {
	var request struct {
		Name   string `json:"name"`
		Public bool   `json:"public"`
	}

	if !utils.DecodeJSONBody(w, r, &request) {
		return
	}

	if request.Name == "" {
		utils.JSONError(w, http.StatusBadRequest, "Bucket name is required")
		return
	}

	err := h.storageService.CreateBucket(request.Name, request.Public)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusCreated, map[string]interface{}{
		"message": "Bucket created successfully",
		"name":    request.Name,
	})
}

// HandleDeleteBucket handles bucket deletion
func (h *StorageHandlers) HandleDeleteBucket(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]

	err := h.storageService.DeleteBucket(bucket)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]string{
		"message": "Bucket deleted successfully",
	})
}

// HandleUploadFile handles file uploads with hook support
func (h *StorageHandlers) HandleUploadFile(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]

	// Parse multipart form
	err := r.ParseMultipartForm(constants.MaxMultipartFormSize)
	if err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Failed to parse form")
		return
	}

	file, header, err := r.FormFile("file")
	if err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Failed to get file")
		return
	}
	defer file.Close()

	// Get parent_folder_id from form (optional)
	parentFolderID := r.FormValue("parent_folder_id")

	// Get content type
	contentType := header.Header.Get("Content-Type")
	if contentType == "" {
		contentType = "application/octet-stream"
	}

	userID := utils.GetUserIDFromRequest(r)

	// Require user authentication for uploads
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Determine the correct bucket
	if utils.IsInternalBucket(bucket) {
		bucket = constants.InternalStorageBucket
	}

	// Prepare hook context for before upload
	if h.hookRegistry != nil {
		hookCtx := &core.HookContext{
			Request:  r,
			Response: w,
			Data: map[string]interface{}{
				"userID":      userID,
				"bucket":      bucket,
				"filename":    header.Filename,
				"fileSize":    header.Size,
				"contentType": contentType,
			},
			Services: nil, // Will be set by registry
		}

		// Execute before upload hooks
		if err := h.hookRegistry.ExecuteHooks(r.Context(), core.HookBeforeUpload, hookCtx); err != nil {
			utils.JSONError(w, http.StatusInsufficientStorage, err.Error())
			return
		}
	}

	// Read file content for upload
	fileContent, err := io.ReadAll(file)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to read file")
		return
	}

	// Upload file using the storage service
	var parentFolderPtr *string
	if parentFolderID != "" {
		parentFolderPtr = &parentFolderID
	}

	object, err := h.storageService.UploadFile(bucket, header.Filename, userID, bytes.NewReader(fileContent), header.Size, contentType, parentFolderPtr)

	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to upload file: "+err.Error())
		return
	}

	// Extract object ID from response
	objectID := ""
	if objMap, ok := object.(map[string]interface{}); ok {
		if id, ok := objMap["id"].(string); ok {
			objectID = id
		}
	}

	// Execute after upload hooks
	if h.hookRegistry != nil {
		hookCtx := &core.HookContext{
			Request:  r,
			Response: w,
			Data: map[string]interface{}{
				"userID":   userID,
				"bucket":   bucket,
				"objectID": objectID,
				"filename": header.Filename,
				"fileSize": header.Size,
			},
			Services: nil,
		}

		// Execute after upload hooks (async)
		go h.hookRegistry.ExecuteHooks(context.Background(), core.HookAfterUpload, hookCtx)
	}

	utils.JSONResponse(w, http.StatusCreated, object)
}

// HandleDeleteObject handles object deletion
func (h *StorageHandlers) HandleDeleteObject(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]
	objectID := vars["id"]

	userID := utils.GetUserIDFromRequest(r)

	// Require user authentication for deletions
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// For internal storage, verify user owns the object
	var err error
	if utils.IsInternalBucket(bucket) {
		// Get object info to verify ownership
		objectInfo, getErr := h.storageService.GetObjectInfo(constants.InternalStorageBucket, objectID)
		if getErr != nil {
			utils.JSONError(w, http.StatusNotFound, "Object not found")
			return
		}

		// Check ownership
		ownershipErr := utils.CheckStorageOwnership(userID, &utils.StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, h.storageService.GetAppID())
		if ownershipErr != nil {
			utils.HandleOwnershipError(w, ownershipErr)
			return
		}

		// Delete from int_storage bucket
		err = h.storageService.DeleteObject(constants.InternalStorageBucket, objectID)
	} else {
		// For other buckets, proceed normally
		err = h.storageService.DeleteObject(bucket, objectID)
	}

	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to delete object: "+err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Object deleted successfully"})
}

// HandleDownloadObject handles file downloads with hook support
func (h *StorageHandlers) HandleDownloadObject(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]
	objectID := vars["id"]

	log.Printf("HandleDownloadObject: bucket=%s, objectID=%s", bucket, objectID)

	userID := utils.GetUserIDFromRequest(r)
	log.Printf("HandleDownloadObject: userID=%s", userID)

	// For internal storage, verify user access
	actualBucket := bucket
	if utils.IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		// Require authentication for internal storage
		if userID == "" {
			utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
			return
		}

		// Get object info to verify ownership
		objectInfo, err := h.storageService.GetObjectInfo(actualBucket, objectID)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "Object not found")
			return
		}

		// Check if object belongs to user
		isOwner := objectInfo.UserID == userID

		// Also check app ID if configured
		if isOwner && h.storageService.GetAppID() != "" {
			isOwner = objectInfo.AppID != nil && *objectInfo.AppID == h.storageService.GetAppID()
		}

		if !isOwner {
			utils.JSONError(w, http.StatusForbidden, "Access denied")
			return
		}
	}

	// Execute before download hooks
	if h.hookRegistry != nil {
		hookCtx := &core.HookContext{
			Request:  r,
			Response: w,
			Data: map[string]interface{}{
				"userID":   userID,
				"bucket":   bucket,
				"objectID": objectID,
			},
			Services: nil,
		}

		if err := h.hookRegistry.ExecuteHooks(r.Context(), core.HookBeforeDownload, hookCtx); err != nil {
			utils.JSONError(w, http.StatusForbidden, err.Error())
			return
		}
	}

	// Get the file from storage service
	reader, filename, contentType, err := h.storageService.GetObject(actualBucket, objectID)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "Object not found")
		return
	}

	// Track bandwidth if we have hooks
	var tracker *BandwidthTracker
	var finalReader io.ReadCloser = reader

	if h.hookRegistry != nil && userID != "" {
		// Wrap reader to track bandwidth
		tracker = &BandwidthTracker{
			reader: reader,
		}
		finalReader = tracker
	}
	defer finalReader.Close()

	// Set headers for download
	w.Header().Set("Content-Type", contentType)
	w.Header().Set("Content-Disposition", "attachment; filename=\""+filename+"\"")

	// Stream the file to the response
	if _, err := io.Copy(w, finalReader); err != nil {
		// Log error but can't send error response as headers are already sent
		log.Printf("Error streaming file: %v", err)
		return
	}

	// Execute after download hooks
	if h.hookRegistry != nil && tracker != nil {
		bytesRead := tracker.bytesRead
		if bytesRead > 0 || userID != "" {
			hookCtx := &core.HookContext{
				Request:  r,
				Response: w,
				Data: map[string]interface{}{
					"userID":    userID,
					"bucket":    bucket,
					"objectID":  objectID,
					"bytesRead": bytesRead,
				},
				Services: nil,
			}

			// Execute after download hooks (async)
			go h.hookRegistry.ExecuteHooks(context.Background(), core.HookAfterDownload, hookCtx)
		}
	}
}

// HandleGetObject handles getting object metadata
func (h *StorageHandlers) HandleGetObject(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]
	objectID := vars["id"]

	userID := utils.GetUserIDFromRequest(r)

	// For internal storage, check access
	actualBucket := bucket
	if utils.IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		// Require authentication for internal storage
		if userID == "" {
			utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
			return
		}
	}

	// Get object info
	objectInfo, err := h.storageService.GetObjectInfo(actualBucket, objectID)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "Object not found")
		return
	}

	// For internal storage, verify user access
	if utils.IsInternalBucket(bucket) {
		ownershipErr := utils.CheckStorageOwnership(userID, &utils.StorageObjectInfo{
			UserID: objectInfo.UserID,
			AppID:  objectInfo.AppID,
		}, h.storageService.GetAppID())
		if ownershipErr != nil {
			utils.HandleOwnershipError(w, ownershipErr)
			return
		}
	}

	utils.JSONResponse(w, http.StatusOK, objectInfo)
}

// HandleRenameObject handles object renaming
func (h *StorageHandlers) HandleRenameObject(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]
	objectID := vars["id"]

	userID := utils.GetUserIDFromRequest(r)

	// Require user authentication for renames
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	var request struct {
		Name string `json:"name"`
	}

	if !utils.DecodeJSONBody(w, r, &request) {
		return
	}

	if request.Name == "" {
		utils.JSONError(w, http.StatusBadRequest, "New name is required")
		return
	}

	// For internal storage, verify user owns the object
	actualBucket := bucket
	if utils.IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		// Get object info to verify ownership
		objectInfo, err := h.storageService.GetObjectInfo(actualBucket, objectID)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "Object not found")
			return
		}

		// Check if object belongs to user
		isOwner := objectInfo.UserID == userID

		// Also check app ID if configured
		if isOwner && h.storageService.GetAppID() != "" {
			isOwner = objectInfo.AppID != nil && *objectInfo.AppID == h.storageService.GetAppID()
		}

		if !isOwner {
			utils.JSONError(w, http.StatusForbidden, "Access denied")
			return
		}
	}

	if err := h.storageService.RenameObject(actualBucket, objectID, request.Name); err != nil {
		utils.JSONError(w, http.StatusInternalServerError, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"message": "Object renamed successfully",
	})
}

// HandleCreateFolder handles folder creation
func (h *StorageHandlers) HandleCreateFolder(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]

	userID := utils.GetUserIDFromRequest(r)

	// Require user authentication for folder creation
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	var request struct {
		Name           string  `json:"name"`
		Path           string  `json:"path"`
		ParentFolderID *string `json:"parentFolderId"`
	}

	if !utils.DecodeJSONBody(w, r, &request) {
		return
	}

	if request.Name == "" {
		utils.JSONError(w, http.StatusBadRequest, "Folder name is required")
		return
	}

	// Determine the correct bucket
	actualBucket := bucket
	if utils.IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket
	}

	// Create the folder with the new method that supports parent_folder_id
	folderID, err := h.storageService.CreateFolderWithParent(actualBucket, request.Name, userID, request.ParentFolderID)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to create folder: "+err.Error())
		return
	}

	// Get the created folder object to return full details
	folder, err := h.storageService.GetObjectInfo(actualBucket, folderID)
	if err != nil {
		utils.JSONResponse(w, http.StatusCreated, map[string]interface{}{
			"id":      folderID,
			"name":    request.Name,
			"message": "Folder created successfully",
		})
		return
	}

	utils.JSONResponse(w, http.StatusCreated, map[string]interface{}{
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

// Presigned URL handlers

// HandleGenerateDownloadURL generates a presigned URL or token for download
func (h *StorageHandlers) HandleGenerateDownloadURL(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]
	objectID := vars["id"]

	userID := utils.GetUserIDFromRequest(r)

	// Get object info
	object, err := h.storageService.GetObjectInfo(bucket, objectID)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "Object not found")
		return
	}

	// Check storage provider
	provider := h.storageService.GetProviderType()

	var response map[string]interface{}

	if provider == "s3" {
		// Generate S3 presigned URL
		url, err := h.storageService.GeneratePresignedDownloadURL(bucket, object.ObjectName, constants.DefaultURLExpiry) // 1 hour
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate download URL")
			return
		}

		// Create download token for tracking
		tokenStr := uuid.New().String()
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}
		fileSize := object.Size

		token, err := h.queries.CreateDownloadToken(r.Context(), db.CreateDownloadTokenParams{
			ID:             uuid.New().String(),
			Token:          tokenStr,
			FileID:         objectID,
			Bucket:         bucket,
			ParentFolderID: object.ParentFolderID,
			ObjectName:     object.ObjectName,
			UserID:         userIDPtr,
			FileSize:       &fileSize,
			ExpiresAt:      apptime.NullTime{Time: apptime.NowTime().Add(apptime.Hour), Valid: true},
		})
		if err != nil {
			log.Printf("Failed to create download token: %v", err)
		}

		response = map[string]interface{}{
			"url":          url,
			"type":         "presigned",
			"expires_in":   constants.DefaultURLExpiry,
			"callback_url": fmt.Sprintf("/api/storage/download-callback/%s", token.Token),
		}
	} else {
		// Generate token for local storage
		tokenStr := uuid.New().String()
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}
		fileSize := object.Size

		token, err := h.queries.CreateDownloadToken(r.Context(), db.CreateDownloadTokenParams{
			ID:             uuid.New().String(),
			Token:          tokenStr,
			FileID:         objectID,
			Bucket:         bucket,
			ParentFolderID: object.ParentFolderID,
			ObjectName:     object.ObjectName,
			UserID:         userIDPtr,
			FileSize:       &fileSize,
			ExpiresAt:      apptime.NullTime{Time: apptime.NowTime().Add(apptime.Hour), Valid: true},
		})
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create download token")
			return
		}

		response = map[string]interface{}{
			"url":        fmt.Sprintf("/api/storage/direct/%s", token.Token),
			"type":       "token",
			"expires_in": constants.DefaultURLExpiry,
		}
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleGenerateUploadURL generates a presigned URL or token for upload
func (h *StorageHandlers) HandleGenerateUploadURL(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]

	var request struct {
		Filename       string  `json:"filename"`
		ParentFolderID *string `json:"parentFolderId,omitempty"`
		ContentType    string  `json:"contentType"`
		MaxSize        int64   `json:"maxSize"`
	}

	if !utils.DecodeJSONBody(w, r, &request) {
		return
	}

	if request.Filename == "" {
		utils.JSONError(w, http.StatusBadRequest, "Filename is required")
		return
	}

	if request.ContentType == "" {
		request.ContentType = "application/octet-stream"
	}

	if request.MaxSize == 0 {
		request.MaxSize = constants.DefaultMaxFileSize
	}

	userID := utils.GetUserIDFromRequest(r)

	// Object key is just the filename now
	objectKey := request.Filename

	// Check storage quota before generating URL
	if h.hookRegistry != nil && userID != "" {
		hookCtx := &core.HookContext{
			Request:  r,
			Response: w,
			Data: map[string]interface{}{
				"userID":   userID,
				"bucket":   bucket,
				"fileSize": request.MaxSize,
			},
			Services: nil,
		}

		if err := h.hookRegistry.ExecuteHooks(r.Context(), core.HookBeforeUpload, hookCtx); err != nil {
			utils.JSONError(w, http.StatusInsufficientStorage, err.Error())
			return
		}
	}

	// Check storage provider
	provider := h.storageService.GetProviderType()

	var response map[string]interface{}

	if provider == "s3" {
		// Generate S3 presigned upload URL
		url, err := h.storageService.GeneratePresignedUploadURL(bucket, objectKey, request.ContentType, constants.DefaultURLExpiry)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate upload URL")
			return
		}

		// Create upload token for tracking
		tokenStr := uuid.New().String()
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}

		token, err := h.queries.CreateUploadToken(r.Context(), db.CreateUploadTokenParams{
			ID:             uuid.New().String(),
			Token:          tokenStr,
			Bucket:         bucket,
			ParentFolderID: request.ParentFolderID,
			ObjectName:     request.Filename,
			UserID:         userIDPtr,
			MaxSize:        &request.MaxSize,
			ContentType:    &request.ContentType,
			ExpiresAt:      apptime.NullTime{Time: apptime.NowTime().Add(apptime.Hour), Valid: true},
		})
		if err != nil {
			log.Printf("Failed to create upload token: %v", err)
		}

		response = map[string]interface{}{
			"url":          url,
			"type":         "presigned",
			"expires_in":   constants.DefaultURLExpiry,
			"callback_url": fmt.Sprintf("/api/storage/upload-callback/%s", token.Token),
		}
	} else {
		// Generate token for local storage
		tokenStr := uuid.New().String()
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}

		token, err := h.queries.CreateUploadToken(r.Context(), db.CreateUploadTokenParams{
			ID:             uuid.New().String(),
			Token:          tokenStr,
			Bucket:         bucket,
			ParentFolderID: request.ParentFolderID,
			ObjectName:     request.Filename,
			UserID:         userIDPtr,
			MaxSize:        &request.MaxSize,
			ContentType:    &request.ContentType,
			ExpiresAt:      apptime.NullTime{Time: apptime.NowTime().Add(apptime.Hour), Valid: true},
		})
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create upload token")
			return
		}

		response = map[string]interface{}{
			"url":        fmt.Sprintf("/api/storage/direct-upload/%s", token.Token),
			"type":       "token",
			"expires_in": constants.DefaultURLExpiry,
		}
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleDirectDownload handles token-based direct download for local storage
func (h *StorageHandlers) HandleDirectDownload(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tokenStr := vars["token"]

	// Get download token
	token, err := h.queries.GetDownloadTokenByToken(r.Context(), tokenStr)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "Invalid or expired token")
		return
	}

	// Check if token is expired
	if token.ExpiresAt.Valid && apptime.NowTime().After(token.ExpiresAt.Time) {
		utils.JSONError(w, http.StatusUnauthorized, "Token has expired")
		return
	}

	// Get the file from storage
	reader, filename, contentType, err := h.storageService.GetObjectByKey(token.Bucket, token.ObjectName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "File not found")
		return
	}

	// Track bandwidth
	tracker := &DirectDownloadTracker{
		reader:  reader,
		tokenID: token.ID,
		queries: h.queries,
		ctx:     r.Context(),
	}
	defer tracker.Close()

	// Set headers for download
	w.Header().Set("Content-Type", contentType)
	w.Header().Set("Content-Disposition", "attachment; filename=\""+filename+"\"")

	// Stream the file
	if _, err := io.Copy(w, tracker); err != nil {
		log.Printf("Error streaming file: %v", err)
	}
}

// HandleDirectUpload handles token-based direct upload for local storage
func (h *StorageHandlers) HandleDirectUpload(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tokenStr := vars["token"]

	// Get upload token
	token, err := h.queries.GetUploadTokenByToken(r.Context(), tokenStr)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "Invalid or expired token")
		return
	}

	// Check if token is expired
	if token.ExpiresAt.Valid && apptime.NowTime().After(token.ExpiresAt.Time) {
		utils.JSONError(w, http.StatusUnauthorized, "Token has expired")
		return
	}

	// Check if already used
	if token.Completed != nil && *token.Completed != 0 {
		utils.JSONError(w, http.StatusConflict, "Token has already been used")
		return
	}

	// Get max size
	maxSize := int64(constants.DefaultMaxFileSize)
	if token.MaxSize != nil {
		maxSize = *token.MaxSize
	}

	// Read file from request body
	fileContent, err := io.ReadAll(io.LimitReader(r.Body, maxSize+1))
	if err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Failed to read file")
		return
	}

	fileSize := int64(len(fileContent))

	// Check file size
	if fileSize > maxSize {
		utils.JSONError(w, http.StatusRequestEntityTooLarge, "File exceeds maximum size")
		return
	}

	// Get userID and contentType
	userID := ""
	if token.UserID != nil {
		userID = *token.UserID
	}
	contentType := "application/octet-stream"
	if token.ContentType != nil {
		contentType = *token.ContentType
	}

	// Upload the file
	object, err := h.storageService.UploadFile(token.Bucket, token.ObjectName, userID, bytes.NewReader(fileContent), fileSize, contentType, nil)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to upload file")
		return
	}

	// Extract object ID from response
	objectIDStr := ""
	if objMap, ok := object.(map[string]interface{}); ok {
		if id, ok := objMap["id"].(string); ok {
			objectIDStr = id
		}
	}

	// Mark token as completed
	now := apptime.NowTime()
	h.queries.CompleteUploadToken(r.Context(), db.CompleteUploadTokenParams{
		ObjectID:    &objectIDStr,
		CompletedAt: apptime.NullTime{Time: now, Valid: true},
		ID:          token.ID,
	})
	h.queries.UpdateUploadTokenProgress(r.Context(), db.UpdateUploadTokenProgressParams{
		BytesUploaded: &fileSize,
		ID:            token.ID,
	})

	// Execute after upload hooks
	if h.hookRegistry != nil && userID != "" {
		hookCtx := &core.HookContext{
			Request:  r,
			Response: w,
			Data: map[string]interface{}{
				"userID":   userID,
				"bucket":   token.Bucket,
				"objectID": objectIDStr,
				"filename": token.ObjectName,
				"key":      token.ObjectName,
				"fileSize": fileSize,
			},
			Services: nil,
		}

		go h.hookRegistry.ExecuteHooks(context.Background(), core.HookAfterUpload, hookCtx)
	}

	utils.JSONResponse(w, http.StatusCreated, object)
}

// BandwidthTracker wraps an io.ReadCloser to track bandwidth usage
type BandwidthTracker struct {
	reader    io.ReadCloser
	bytesRead int64
}

func (b *BandwidthTracker) Read(p []byte) (n int, err error) {
	n, err = b.reader.Read(p)
	b.bytesRead += int64(n)
	return n, err
}

func (b *BandwidthTracker) Close() error {
	return b.reader.Close()
}

// DirectDownloadTracker tracks bandwidth for direct downloads
type DirectDownloadTracker struct {
	reader    io.ReadCloser
	tokenID   string
	queries   *db.Queries
	ctx       context.Context
	bytesRead int64
}

func (d *DirectDownloadTracker) Read(p []byte) (n int, err error) {
	n, err = d.reader.Read(p)
	d.bytesRead += int64(n)
	return n, err
}

func (d *DirectDownloadTracker) Close() error {
	// Update token with bytes served
	d.queries.UpdateDownloadTokenProgress(d.ctx, db.UpdateDownloadTokenProgressParams{
		BytesServed: &d.bytesRead,
		ID:          d.tokenID,
	})

	// Mark as complete
	now := apptime.NowTime()
	d.queries.CompleteDownloadToken(d.ctx, db.CompleteDownloadTokenParams{
		CallbackAt: apptime.NullTime{Time: now, Valid: true},
		ID:         d.tokenID,
	})

	return d.reader.Close()
}

// HandleGetStorageQuota returns storage quota information for the current user
func (h *StorageHandlers) HandleGetStorageQuota(w http.ResponseWriter, r *http.Request) {
	userID := utils.GetUserIDFromRequest(r)
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Note: If CloudStorage extension is enabled, it provides additional quota functionality
	// through its own endpoints at /ext/cloudstorage/api/quota

	// Get storage used by user
	storageUsed, err := h.storageService.GetUserStorageUsed(userID)
	if err != nil {
		log.Printf("Failed to get user storage: %v", err)
		storageUsed = 0
	}

	// Default quota limits (can be overridden by CloudStorage extension)
	maxStorage := int64(constants.DefaultMaxStorageBytes)
	maxBandwidth := int64(constants.DefaultMaxBandwidthBytes)

	// Check if there's a storage quota record for this user
	var quotaMaxStorage, quotaMaxBandwidth, quotaStorageUsed, quotaBandwidthUsed int64

	// Try to get quota from ext_cloudstorage_storage_quotas table if it exists
	row := h.sqlDB.QueryRowContext(r.Context(), `
		SELECT
			COALESCE(max_storage_bytes, ?) as max_storage_bytes,
			COALESCE(max_bandwidth_bytes, ?) as max_bandwidth_bytes,
			COALESCE(storage_used, 0) as storage_used,
			COALESCE(bandwidth_used, 0) as bandwidth_used
		FROM ext_cloudstorage_storage_quotas
		WHERE user_id = ?
	`, maxStorage, maxBandwidth, userID)

	err = row.Scan(&quotaMaxStorage, &quotaMaxBandwidth, &quotaStorageUsed, &quotaBandwidthUsed)
	if err != nil {
		// Table doesn't exist or user has no quota record, use defaults
		quotaMaxStorage = maxStorage
		quotaMaxBandwidth = maxBandwidth
		quotaStorageUsed = storageUsed
		quotaBandwidthUsed = 0
	}

	// Calculate percentage
	percentage := float64(0)
	if quotaMaxStorage > 0 {
		percentage = (float64(quotaStorageUsed) / float64(quotaMaxStorage)) * 100
	}

	response := map[string]interface{}{
		"used":            quotaStorageUsed,
		"total":           quotaMaxStorage,
		"percentage":      percentage,
		"storage_used":    quotaStorageUsed,
		"storage_limit":   quotaMaxStorage,
		"bandwidth_used":  quotaBandwidthUsed,
		"bandwidth_limit": quotaMaxBandwidth,
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleGetStorageStats returns storage statistics for the current user
func (h *StorageHandlers) HandleGetStorageStats(w http.ResponseWriter, r *http.Request) {
	userID := utils.GetUserIDFromRequest(r)
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Get storage statistics
	stats, err := h.storageService.GetStorageStats(userID)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to get storage statistics")
		return
	}

	// Format response
	response := map[string]interface{}{
		"totalSize":     stats["totalSize"],
		"fileCount":     stats["fileCount"],
		"folderCount":   stats["folderCount"],
		"sharedCount":   stats["sharedCount"],
		"recentUploads": stats["recentUploads"],
		// Additional fields for compatibility
		"trashedCount": 0, // We don't have trash functionality yet
		"totalFiles":   stats["fileCount"],
		"totalFolders": stats["folderCount"],
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleGetAdminStorageStats returns storage statistics for all users (admin only)
func (h *StorageHandlers) HandleGetAdminStorageStats(w http.ResponseWriter, r *http.Request) {
	// TODO: Add admin role check here

	stats, err := h.storageService.GetAllUsersStorageStats()
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to get storage statistics")
		return
	}

	utils.JSONResponse(w, http.StatusOK, stats)
}

// HandleGetRecentlyViewed returns recently viewed items for the current user
func (h *StorageHandlers) HandleGetRecentlyViewed(w http.ResponseWriter, r *http.Request) {
	userID := utils.GetUserIDFromRequest(r)
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Get limit from query params, default to 30
	limit := 30
	if limitStr := r.URL.Query().Get("limit"); limitStr != "" {
		if l, err := strconv.Atoi(limitStr); err == nil && l > 0 && l <= 100 {
			limit = l
		}
	}

	objects, err := h.storageService.GetRecentlyViewed(userID, limit)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to get recently viewed items")
		return
	}

	var response []map[string]interface{}
	for _, obj := range objects {
		response = append(response, map[string]interface{}{
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

	if response == nil {
		response = []map[string]interface{}{}
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleUpdateLastViewed updates the last viewed timestamp for an item
func (h *StorageHandlers) HandleUpdateLastViewed(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	itemID := vars["id"]

	userID := utils.GetUserIDFromRequest(r)
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	err := h.storageService.UpdateLastViewed(itemID, userID)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to update last viewed")
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"message":     "Last viewed updated successfully",
		"last_viewed": apptime.NowTime(),
	})
}

// HandleSearchStorageObjects searches for storage objects by name
func (h *StorageHandlers) HandleSearchStorageObjects(w http.ResponseWriter, r *http.Request) {
	userID := utils.GetUserIDFromRequest(r)
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Get search query
	query := r.URL.Query().Get("q")
	if query == "" {
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"items": []interface{}{},
		})
		return
	}

	// Get app_id from query params
	appID := r.URL.Query().Get("app_id")
	if appID == "" {
		appID = "default"
	}

	objects, err := h.storageService.SearchStorageObjects(userID, appID, query, 50)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to search items")
		return
	}

	var result []map[string]interface{}
	for _, obj := range objects {
		result = append(result, map[string]interface{}{
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

	if result == nil {
		result = []map[string]interface{}{}
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"items": result,
	})
}

// HandleUpdateObjectMetadata handles updating object metadata
func (h *StorageHandlers) HandleUpdateObjectMetadata(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]
	objectID := vars["id"]

	userID := utils.GetUserIDFromRequest(r)

	// Require user authentication
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	var request struct {
		Metadata string `json:"metadata"`
	}

	if !utils.DecodeJSONBody(w, r, &request) {
		return
	}

	// For internal storage, verify user owns the object
	actualBucket := bucket
	if utils.IsInternalBucket(bucket) {
		actualBucket = constants.InternalStorageBucket

		// Get object info to verify ownership
		objectInfo, err := h.storageService.GetObjectInfo(actualBucket, objectID)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "Object not found")
			return
		}

		// Check if object belongs to user
		isOwner := objectInfo.UserID == userID

		// Also check app ID if configured
		if isOwner && h.storageService.GetAppID() != "" {
			isOwner = objectInfo.AppID != nil && *objectInfo.AppID == h.storageService.GetAppID()
		}

		if !isOwner {
			utils.JSONError(w, http.StatusForbidden, "Access denied")
			return
		}
	}

	// Update the metadata field in the database
	if err := h.storageService.UpdateObjectMetadata(actualBucket, objectID, request.Metadata); err != nil {
		utils.JSONError(w, http.StatusInternalServerError, err.Error())
		return
	}

	// Get updated object info to return
	objectInfo, err := h.storageService.GetObjectInfo(actualBucket, objectID)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to retrieve updated object")
		return
	}

	utils.JSONResponse(w, http.StatusOK, objectInfo)
}

