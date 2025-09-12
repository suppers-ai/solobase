package api

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/models"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
	pkgstorage "github.com/suppers-ai/storage"
)

// StorageHandlers contains all storage-related handlers with hook support
type StorageHandlers struct {
	storageService *services.StorageService
	db             *database.DB
	hookRegistry   *core.ExtensionRegistry
}

// extractUserIDFromToken extracts user ID from JWT token in Authorization header
func extractUserIDFromToken(r *http.Request) string {
	authHeader := r.Header.Get("Authorization")
	if authHeader == "" {
		return ""
	}

	tokenString := strings.TrimPrefix(authHeader, "Bearer ")
	if tokenString == authHeader {
		return ""
	}

	claims := &Claims{}
	token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
		// Use the same secret as in middleware.go
		return jwtSecret, nil
	})

	if err != nil || !token.Valid {
		return ""
	}

	return claims.UserID
}

// NewStorageHandlers creates new storage handlers with hook support
func NewStorageHandlers(storageService *services.StorageService, db *database.DB, hookRegistry *core.ExtensionRegistry) *StorageHandlers {
	return &StorageHandlers{
		storageService: storageService,
		db:             db,
		hookRegistry:   hookRegistry,
	}
}

// HandleGetStorageBuckets handles bucket listing
func (h *StorageHandlers) HandleGetStorageBuckets(w http.ResponseWriter, r *http.Request) {
	buckets, err := h.storageService.GetBuckets()
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch buckets")
		return
	}

	utils.JSONResponse(w, http.StatusOK, buckets)
}

// HandleGetBucketObjects handles object listing in a bucket
func (h *StorageHandlers) HandleGetBucketObjects(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	bucket := vars["bucket"]

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// Get parent folder ID from query
	parentFolderIDStr := r.URL.Query().Get("parent_folder_id")
	var parentFolderID *string
	if parentFolderIDStr != "" {
		parentFolderID = &parentFolderIDStr
	}

	// For user files, ensure we have a user ID
	if bucket == "user-files" || bucket == "int_storage" {
		bucket = "int_storage"
		if userID == "" {
			// No user ID means no access to user files
			utils.JSONResponse(w, http.StatusOK, []interface{}{})
			return
		}
	}

	// Get objects filtered by userID, appID, and parentFolderID
	objects, err := h.storageService.GetObjects(bucket, userID, parentFolderID)
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

	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
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
	err := r.ParseMultipartForm(32 << 20) // 32MB max
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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// Require user authentication for uploads
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Determine the correct bucket
	if bucket == "user-files" || bucket == "int_storage" {
		bucket = "int_storage"
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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// Require user authentication for deletions
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// For internal storage, verify user owns the object
	var err error
	if bucket == "user-files" || bucket == "int_storage" {
		// Get object info to verify ownership
		objectInfo, getErr := h.storageService.GetObjectInfo("int_storage", objectID)
		if getErr != nil {
			utils.JSONError(w, http.StatusNotFound, "Object not found")
			return
		}

		// Check if object belongs to the user
		if objectInfo.UserID != userID {
			utils.JSONError(w, http.StatusForbidden, "Access denied")
			return
		}

		// Check if object belongs to the correct app
		if h.storageService.GetAppID() != "" {
			if objectInfo.AppID == nil || *objectInfo.AppID != h.storageService.GetAppID() {
				utils.JSONError(w, http.StatusForbidden, "Access denied")
				return
			}
		}

		// Delete from int_storage bucket
		err = h.storageService.DeleteObject("int_storage", objectID)
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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}
	log.Printf("HandleDownloadObject: userID=%s", userID)

	// For internal storage, verify user access
	actualBucket := bucket
	if bucket == "user-files" || bucket == "int_storage" {
		actualBucket = "int_storage"

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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// For internal storage, check access
	actualBucket := bucket
	if bucket == "user-files" || bucket == "int_storage" {
		actualBucket = "int_storage"

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
	if bucket == "user-files" || bucket == "int_storage" {
		// Check if object belongs to user
		fullPath := objectInfo.ObjectName
		expectedPrefix := fmt.Sprintf("%s/%s/", userID, h.storageService.GetAppID())

		// Check ownership: path starts with userID/appID
		isOwner := strings.HasPrefix(fullPath, expectedPrefix)

		// Also check if the object's UserID field matches (for backward compatibility)
		if objectInfo.UserID == userID {
			isOwner = true
		}

		if !isOwner {
			utils.JSONError(w, http.StatusForbidden, "Access denied")
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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// Require user authentication for renames
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	var request struct {
		Name string `json:"name"`
	}

	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if request.Name == "" {
		utils.JSONError(w, http.StatusBadRequest, "New name is required")
		return
	}

	// For internal storage, verify user owns the object
	actualBucket := bucket
	if bucket == "user-files" || bucket == "int_storage" {
		actualBucket = "int_storage"

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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// Require user authentication for folder creation
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	var request struct {
		Name           string  `json:"name"`
		Path           string  `json:"path"`
		ParentFolderID *string `json:"parent_folder_id"`
	}

	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if request.Name == "" {
		utils.JSONError(w, http.StatusBadRequest, "Folder name is required")
		return
	}

	// Determine the correct bucket
	actualBucket := bucket
	if bucket == "user-files" || bucket == "int_storage" {
		actualBucket = "int_storage"
	}

	// Create the folder with the new method that supports parent_folder_id
	folderID, err := h.storageService.CreateFolderWithParent(actualBucket, request.Name, userID, request.ParentFolderID)
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to create folder: "+err.Error())
		return
	}

	// Get the created folder object to return full details
	var folder pkgstorage.StorageObject
	if err := h.storageService.GetDB().Where("id = ?", folderID).First(&folder).Error; err != nil {
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

	// Get user ID from context
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

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
		url, err := h.storageService.GeneratePresignedDownloadURL(bucket, object.ObjectName, 3600) // 1 hour
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate download URL")
			return
		}

		// Create download token for tracking
		token := &models.DownloadToken{
			ID:             uuid.New().String(),
			Token:          uuid.New().String(),
			FileID:         objectID,
			Bucket:         bucket,
			ParentFolderID: object.ParentFolderID,
			ObjectName:     object.ObjectName,
			UserID:         userID,
			FileSize:       object.Size,
			ExpiresAt:      time.Now().Add(time.Hour),
		}

		if err := h.db.Create(token).Error; err != nil {
			log.Printf("Failed to create download token: %v", err)
		}

		response = map[string]interface{}{
			"url":          url,
			"type":         "presigned",
			"expires_in":   3600,
			"callback_url": fmt.Sprintf("/api/storage/download-callback/%s", token.Token),
		}
	} else {
		// Generate token for local storage
		token := &models.DownloadToken{
			ID:             uuid.New().String(),
			Token:          uuid.New().String(),
			FileID:         objectID,
			Bucket:         bucket,
			ParentFolderID: object.ParentFolderID,
			ObjectName:     object.ObjectName,
			UserID:         userID,
			FileSize:       object.Size,
			ExpiresAt:      time.Now().Add(time.Hour),
		}

		if err := h.db.Create(token).Error; err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create download token")
			return
		}

		response = map[string]interface{}{
			"url":        fmt.Sprintf("/api/storage/direct/%s", token.Token),
			"type":       "token",
			"expires_in": 3600,
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
		ParentFolderID *string `json:"parent_folder_id,omitempty"`
		ContentType    string  `json:"contentType"`
		MaxSize        int64   `json:"maxSize"`
	}

	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
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
		request.MaxSize = 10 << 20 // 10MB default
	}

	// Get user ID from context
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

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
		url, err := h.storageService.GeneratePresignedUploadURL(bucket, objectKey, request.ContentType, 3600)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to generate upload URL")
			return
		}

		// Create upload token for tracking
		objectName := request.Filename

		token := &models.UploadToken{
			ID:             uuid.New().String(),
			Token:          uuid.New().String(),
			Bucket:         bucket,
			ParentFolderID: request.ParentFolderID,
			ObjectName:     objectName,
			UserID:         userID,
			MaxSize:        request.MaxSize,
			ContentType:    request.ContentType,
			ExpiresAt:      time.Now().Add(time.Hour),
		}

		if err := h.db.Create(token).Error; err != nil {
			log.Printf("Failed to create upload token: %v", err)
		}

		response = map[string]interface{}{
			"url":          url,
			"type":         "presigned",
			"expires_in":   3600,
			"callback_url": fmt.Sprintf("/api/storage/upload-callback/%s", token.Token),
		}
	} else {
		// Generate token for local storage
		objectName := request.Filename

		token := &models.UploadToken{
			ID:             uuid.New().String(),
			Token:          uuid.New().String(),
			Bucket:         bucket,
			ParentFolderID: request.ParentFolderID,
			ObjectName:     objectName,
			UserID:         userID,
			MaxSize:        request.MaxSize,
			ContentType:    request.ContentType,
			ExpiresAt:      time.Now().Add(time.Hour),
		}

		if err := h.db.Create(token).Error; err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create upload token")
			return
		}

		response = map[string]interface{}{
			"url":        fmt.Sprintf("/api/storage/direct-upload/%s", token.Token),
			"type":       "token",
			"expires_in": 3600,
		}
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleDirectDownload handles token-based direct download for local storage
func (h *StorageHandlers) HandleDirectDownload(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tokenStr := vars["token"]

	// Get download token
	var token models.DownloadToken
	if err := h.db.Where("token = ?", tokenStr).First(&token).Error; err != nil {
		utils.JSONError(w, http.StatusNotFound, "Invalid or expired token")
		return
	}

	// Check if token is expired
	if time.Now().After(token.ExpiresAt) {
		utils.JSONError(w, http.StatusUnauthorized, "Token has expired")
		return
	}

	// Get the file from storage
	// Reconstruct the full path from ObjectPath and ObjectName
	fullPath := token.ObjectName
	if false { // ObjectPath no longer used
		fullPath = "" + "/" + token.ObjectName
	}
	reader, filename, contentType, err := h.storageService.GetObjectByKey(token.Bucket, fullPath)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, "File not found")
		return
	}

	// Track bandwidth
	tracker := &DirectDownloadTracker{
		reader: reader,
		token:  &token,
		db:     h.db,
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
	var token models.UploadToken
	if err := h.db.Where("token = ?", tokenStr).First(&token).Error; err != nil {
		utils.JSONError(w, http.StatusNotFound, "Invalid or expired token")
		return
	}

	// Check if token is expired
	if time.Now().After(token.ExpiresAt) {
		utils.JSONError(w, http.StatusUnauthorized, "Token has expired")
		return
	}

	// Check if already used
	if token.Completed {
		utils.JSONError(w, http.StatusConflict, "Token has already been used")
		return
	}

	// Read file from request body
	fileContent, err := io.ReadAll(io.LimitReader(r.Body, token.MaxSize+1))
	if err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Failed to read file")
		return
	}

	fileSize := int64(len(fileContent))

	// Check file size
	if fileSize > token.MaxSize {
		utils.JSONError(w, http.StatusRequestEntityTooLarge, "File exceeds maximum size")
		return
	}

	// Upload the file
	// Reconstruct the full path from ObjectPath and ObjectName
	fullPath := token.ObjectName
	if false { // ObjectPath no longer used
		fullPath = "" + "/" + token.ObjectName
	}
	object, err := h.storageService.UploadFile(token.Bucket, fullPath, token.UserID, bytes.NewReader(fileContent), fileSize, token.ContentType, nil)
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
	token.Completed = true
	token.BytesUploaded = fileSize
	token.ObjectID = objectIDStr
	h.db.Save(&token)

	// Execute after upload hooks
	if h.hookRegistry != nil && token.UserID != "" {
		hookCtx := &core.HookContext{
			Request:  r,
			Response: w,
			Data: map[string]interface{}{
				"userID":   token.UserID,
				"bucket":   token.Bucket,
				"objectID": objectIDStr,
				"filename": token.ObjectName,
				"key":      fullPath,
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
	token     *models.DownloadToken
	db        *database.DB
	bytesRead int64
}

func (d *DirectDownloadTracker) Read(p []byte) (n int, err error) {
	n, err = d.reader.Read(p)
	d.bytesRead += int64(n)
	return n, err
}

func (d *DirectDownloadTracker) Close() error {
	// Update token with bytes served
	d.token.BytesServed = d.bytesRead
	d.token.Completed = (d.bytesRead >= d.token.FileSize)
	d.db.Save(d.token)

	// Execute after download hooks if we have the registry
	// Note: This would need the hook registry to be accessible

	return d.reader.Close()
}

// HandleGetStorageQuota returns storage quota information for the current user
func (h *StorageHandlers) HandleGetStorageQuota(w http.ResponseWriter, r *http.Request) {
	userID := extractUserIDFromToken(r)
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
	maxStorage := int64(5 * 1024 * 1024 * 1024)    // 5GB default
	maxBandwidth := int64(10 * 1024 * 1024 * 1024) // 10GB default

	// Check if there's a storage quota record for this user
	var quota struct {
		MaxStorageBytes   int64 `json:"max_storage_bytes"`
		MaxBandwidthBytes int64 `json:"max_bandwidth_bytes"`
		StorageUsed       int64 `json:"storage_used"`
		BandwidthUsed     int64 `json:"bandwidth_used"`
	}

	// Try to get quota from ext_cloudstorage_storage_quotas table if it exists
	err = h.db.Raw(`
		SELECT 
			COALESCE(max_storage_bytes, ?) as max_storage_bytes,
			COALESCE(max_bandwidth_bytes, ?) as max_bandwidth_bytes,
			COALESCE(storage_used, 0) as storage_used,
			COALESCE(bandwidth_used, 0) as bandwidth_used
		FROM ext_cloudstorage_storage_quotas 
		WHERE user_id = ?
	`, maxStorage, maxBandwidth, userID).Scan(&quota).Error

	if err != nil {
		// Table doesn't exist or user has no quota record, use defaults
		quota.MaxStorageBytes = maxStorage
		quota.MaxBandwidthBytes = maxBandwidth
		quota.StorageUsed = storageUsed
		quota.BandwidthUsed = 0
	}

	// Calculate percentage
	percentage := float64(0)
	if quota.MaxStorageBytes > 0 {
		percentage = (float64(quota.StorageUsed) / float64(quota.MaxStorageBytes)) * 100
	}

	response := map[string]interface{}{
		"used":            quota.StorageUsed,
		"total":           quota.MaxStorageBytes,
		"percentage":      percentage,
		"storage_used":    quota.StorageUsed,
		"storage_limit":   quota.MaxStorageBytes,
		"bandwidth_used":  quota.BandwidthUsed,
		"bandwidth_limit": quota.MaxBandwidthBytes,
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleGetStorageStats returns storage statistics for the current user
func (h *StorageHandlers) HandleGetStorageStats(w http.ResponseWriter, r *http.Request) {
	userID := extractUserIDFromToken(r)
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
		"totalSize":     stats["total_size"],
		"fileCount":     stats["file_count"],
		"folderCount":   stats["folder_count"],
		"sharedCount":   stats["shared_count"],
		"recentUploads": stats["recent_uploads"],
		// Additional fields for compatibility
		"trashedCount": 0, // We don't have trash functionality yet
		"totalFiles":   stats["file_count"],
		"totalFolders": stats["folder_count"],
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
	userID := extractUserIDFromToken(r)
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

	// Get recently viewed items from database
	var items []pkgstorage.StorageObject
	query := h.storageService.GetDB().
		Where("user_id = ? AND last_viewed IS NOT NULL", userID).
		Order("last_viewed DESC").
		Limit(limit)

	if err := query.Find(&items).Error; err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to get recently viewed items")
		return
	}

	// Return raw StorageObject data
	response := make([]map[string]interface{}, len(items))
	for i, item := range items {
		response[i] = map[string]interface{}{
			"id":               item.ID,
			"bucket_name":      item.BucketName,
			"object_name":      item.ObjectName,
			"parent_folder_id": item.ParentFolderID,
			"size":             item.Size,
			"content_type":     item.ContentType,
			"checksum":         item.Checksum,
			"metadata":         item.Metadata,
			"created_at":       item.CreatedAt,
			"updated_at":       item.UpdatedAt,
			"last_viewed":      item.LastViewed,
			"user_id":          item.UserID,
			"app_id":           item.AppID,
		}
	}

	utils.JSONResponse(w, http.StatusOK, response)
}

// HandleUpdateLastViewed updates the last viewed timestamp for an item
func (h *StorageHandlers) HandleUpdateLastViewed(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	itemID := vars["id"]

	userID := extractUserIDFromToken(r)
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	// Update last_viewed timestamp
	now := time.Now()
	if err := h.storageService.GetDB().
		Model(&pkgstorage.StorageObject{}).
		Where("id = ? AND user_id = ?", itemID, userID).
		Update("last_viewed", now).Error; err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to update last viewed")
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"message":     "Last viewed updated successfully",
		"last_viewed": now,
	})
}

// HandleSearchStorageObjects searches for storage objects by name
func (h *StorageHandlers) HandleSearchStorageObjects(w http.ResponseWriter, r *http.Request) {
	userID := extractUserIDFromToken(r)
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

	// Search for items matching the query
	var items []pkgstorage.StorageObject
	searchPattern := "%" + query + "%"

	dbQuery := h.storageService.GetDB().
		Where("user_id = ? AND app_id = ? AND name LIKE ?", userID, appID, searchPattern).
		Order("updated_at DESC").
		Limit(50)

	if err := dbQuery.Find(&items).Error; err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to search items")
		return
	}

	// Return raw StorageObject data
	var result []map[string]interface{}
	for _, item := range items {
		result = append(result, map[string]interface{}{
			"id":               item.ID,
			"bucket_name":      item.BucketName,
			"object_name":      item.ObjectName,
			"parent_folder_id": item.ParentFolderID,
			"size":             item.Size,
			"content_type":     item.ContentType,
			"checksum":         item.Checksum,
			"metadata":         item.Metadata,
			"created_at":       item.CreatedAt,
			"updated_at":       item.UpdatedAt,
			"last_viewed":      item.LastViewed,
			"user_id":          item.UserID,
			"app_id":           item.AppID,
		})
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

	// Get user ID from context if available, otherwise try to extract from token
	userID, _ := r.Context().Value("user_id").(string)
	if userID == "" {
		userID = extractUserIDFromToken(r)
	}

	// Require user authentication
	if userID == "" {
		utils.JSONError(w, http.StatusUnauthorized, "Authentication required")
		return
	}

	var request struct {
		Metadata string `json:"metadata"`
	}

	if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
		utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	// For internal storage, verify user owns the object
	actualBucket := bucket
	if bucket == "user-files" || bucket == "int_storage" {
		actualBucket = "int_storage"

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

// Helper function to determine item type
func getItemType(item pkgstorage.StorageObject) string {
	if item.ContentType == "application/x-directory" {
		return "folder"
	}
	return "file"
}
