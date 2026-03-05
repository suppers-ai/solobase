package files

import (
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"strings"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/constants"
	"github.com/wafer-run/wafer-go/services/database"
)

// Storage ownership errors
var (
	ErrNotOwner       = errors.New("access denied: not owner")
	ErrAppIDMismatch  = errors.New("access denied: app ID mismatch")
	ErrObjectNotFound = errors.New("object not found")
)

// NormalizeBucket converts user-facing bucket names to internal bucket names
func NormalizeBucket(bucket string) string {
	if bucket == constants.UserFilesBucket || bucket == constants.InternalStorageBucket {
		return constants.InternalStorageBucket
	}
	return bucket
}

// IsInternalBucket checks if a bucket is the internal storage bucket
func IsInternalBucket(bucket string) bool {
	return bucket == constants.UserFilesBucket || bucket == constants.InternalStorageBucket
}

// StorageObjectInfo holds the basic info needed for ownership checks
type StorageObjectInfo struct {
	UserID string
	AppID  *string
}

// CheckStorageOwnership verifies that the user owns the storage object
func CheckStorageOwnership(userID string, obj *StorageObjectInfo, expectedAppID string) error {
	if obj == nil {
		return ErrObjectNotFound
	}
	if obj.UserID != userID {
		return ErrNotOwner
	}
	if expectedAppID != "" {
		if obj.AppID == nil || *obj.AppID != expectedAppID {
			return ErrAppIDMismatch
		}
	}
	return nil
}

// nilIfEmpty returns nil if string is empty, otherwise returns pointer to string.
func nilIfEmpty(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

// strPtr returns a pointer to a string, or nil if empty
func strPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

// stringPtrOrNil returns a pointer to a string, or nil if empty
func stringPtrOrNil(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

// stringVal extracts a string from an interface value
func stringVal(v any) string {
	if v == nil {
		return ""
	}
	switch val := v.(type) {
	case string:
		return val
	case *string:
		if val != nil {
			return *val
		}
		return ""
	default:
		return fmt.Sprintf("%v", v)
	}
}

// toInt64Val extracts an int64 from an interface value
func toInt64Val(v any) int64 {
	if v == nil {
		return 0
	}
	switch val := v.(type) {
	case int64:
		return val
	case int:
		return int64(val)
	case int32:
		return int64(val)
	case float64:
		return int64(val)
	case float32:
		return int64(val)
	default:
		return 0
	}
}

// toBoolVal converts an interface value to bool
func toBoolVal(v any) bool {
	if v == nil {
		return false
	}
	switch val := v.(type) {
	case bool:
		return val
	case int64:
		return val != 0
	case int:
		return val != 0
	case float64:
		return val != 0
	case string:
		return val == "1" || val == "true"
	default:
		return false
	}
}

// parseTime parses a time value from an interface
func parseTime(v any) apptime.Time {
	if v == nil {
		return apptime.Time{}
	}
	s := stringVal(v)
	if s == "" {
		return apptime.Time{}
	}
	return apptime.MustParse(s)
}

// nullTimeToAny converts a NullTime to an any value for DB storage
func nullTimeToAny(nt apptime.NullTime) any {
	if !nt.Valid {
		return nil
	}
	return apptime.Format(nt.Time)
}

// setOptional sets a map entry if the pointer is non-nil.
func setOptional[T any](m map[string]any, key string, ptr *T) {
	if ptr != nil {
		m[key] = *ptr
	}
}

// formatBytes formats bytes to human-readable string
func formatBytes(bytes int64) string {
	if bytes == 0 {
		return "0 B"
	}
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(bytes)/float64(div), "KMGTPE"[exp])
}

// getFileType returns a human-readable file type based on extension
func getFileType(filename string) string {
	ext := ""
	if idx := strings.LastIndex(filename, "."); idx != -1 {
		ext = filename[idx+1:]
	}

	switch ext {
	case "jpg", "jpeg", "png", "gif", "webp", "svg":
		return "image"
	case "pdf":
		return "pdf"
	case "mp4", "avi", "mov", "webm":
		return "video"
	case "mp3", "wav", "ogg", "m4a":
		return "audio"
	case "zip", "tar", "gz", "rar", "7z":
		return "archive"
	case "json", "js", "ts", "go", "py", "html", "css":
		return "code"
	default:
		return "file"
	}
}

// getFileExtension extracts the file extension from a filename
func getFileExtension(fileName string) string {
	parts := strings.Split(fileName, ".")
	if len(parts) > 1 {
		return parts[len(parts)-1]
	}
	return ""
}

// parseIPAddress extracts IP from address string
func parseIPAddress(addr string) string {
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		return addr
	}
	return host
}

// comparePermissionLevel returns 1 if a > b, -1 if a < b, 0 if equal
func comparePermissionLevel(a, b PermissionLevel) int {
	levels := map[PermissionLevel]int{
		PermissionView:  1,
		PermissionEdit:  2,
		PermissionAdmin: 3,
	}
	aLevel := levels[a]
	bLevel := levels[b]
	if aLevel > bLevel {
		return 1
	} else if aLevel < bLevel {
		return -1
	}
	return 0
}

// mergeExtensions merges two comma-separated extension lists
func mergeExtensions(ext1, ext2 string) string {
	if ext1 == "" {
		return ext2
	}
	if ext2 == "" {
		return ext1
	}
	allExts := make(map[string]bool)
	for _, ext := range strings.Split(ext1, ",") {
		allExts[strings.TrimSpace(ext)] = true
	}
	for _, ext := range strings.Split(ext2, ",") {
		allExts[strings.TrimSpace(ext)] = true
	}
	result := []string{}
	for ext := range allExts {
		if ext != "" {
			result = append(result, ext)
		}
	}
	return strings.Join(result, ",")
}

// intersectExtensions keeps only extensions present in both lists
func intersectExtensions(ext1, ext2 string) string {
	exts1 := make(map[string]bool)
	for _, ext := range strings.Split(ext1, ",") {
		exts1[strings.TrimSpace(ext)] = true
	}
	result := []string{}
	for _, ext := range strings.Split(ext2, ",") {
		ext = strings.TrimSpace(ext)
		if exts1[ext] {
			result = append(result, ext)
		}
	}
	return strings.Join(result, ",")
}

// Record conversion helpers

func recordToAccessLog(rec *database.Record) StorageAccessLog {
	d := rec.Data
	log := StorageAccessLog{
		ID:        rec.ID,
		ObjectID:  stringVal(d["object_id"]),
		Action:    StorageAction(stringVal(d["action"])),
		CreatedAt: parseTime(d["created_at"]),
	}
	if v := d["user_id"]; v != nil {
		s := stringVal(v)
		if s != "" {
			log.UserID = &s
		}
	}
	if v := d["ip_address"]; v != nil {
		s := stringVal(v)
		if s != "" {
			log.IPAddress = &s
		}
	}
	if v := d["user_agent"]; v != nil {
		s := stringVal(v)
		if s != "" {
			log.UserAgent = &s
		}
	}
	if v := d["metadata"]; v != nil {
		s := stringVal(v)
		if s != "" {
			log.Metadata = json.RawMessage(s)
		}
	}
	return log
}

func recordToShareWithObject(rec *database.Record) StorageShareWithObject {
	share := recordToStorageShare(rec)
	d := rec.Data
	swo := StorageShareWithObject{
		StorageShare:    share,
		ObjectName:      stringVal(d["object_name"]),
		ContentType:     stringVal(d["content_type"]),
		Size:            toInt64Val(d["size"]),
		ObjectCreatedAt: parseTime(d["object_created_at"]),
	}
	if v := d["object_metadata"]; v != nil {
		s := stringVal(v)
		if s != "" {
			raw := json.RawMessage(s)
			swo.ObjectMetadata = &raw
		}
	}
	return swo
}

func recordToStorageObject(rec *database.Record) *storageObject {
	d := rec.Data
	obj := &storageObject{
		ID:          rec.ID,
		BucketName:  stringVal(d["bucket_name"]),
		ObjectName:  stringVal(d["object_name"]),
		Size:        toInt64Val(d["size"]),
		ContentType: stringVal(d["content_type"]),
		Checksum:    stringVal(d["checksum"]),
		Metadata:    stringVal(d["metadata"]),
		CreatedAt:   parseTime(d["created_at"]),
		UpdatedAt:   parseTime(d["updated_at"]),
		UserID:      stringVal(d["user_id"]),
	}
	if v := d["parent_folder_id"]; v != nil {
		s := stringVal(v)
		if s != "" {
			obj.ParentFolderID = &s
		}
	}
	if v := d["app_id"]; v != nil {
		s := stringVal(v)
		if s != "" {
			obj.AppID = &s
		}
	}
	return obj
}

func recordToStorageQuota(rec *database.Record) StorageQuota {
	d := rec.Data
	q := StorageQuota{
		ID:                rec.ID,
		UserID:            stringVal(d["user_id"]),
		MaxStorageBytes:   toInt64Val(d["max_storage_bytes"]),
		MaxBandwidthBytes: toInt64Val(d["max_bandwidth_bytes"]),
		StorageUsed:       toInt64Val(d["storage_used"]),
		BandwidthUsed:     toInt64Val(d["bandwidth_used"]),
		CreatedAt:         parseTime(d["created_at"]),
		UpdatedAt:         parseTime(d["updated_at"]),
	}
	if v := d["reset_bandwidth_at"]; v != nil {
		s := stringVal(v)
		if s != "" {
			t := apptime.MustParse(s)
			q.ResetBandwidthAt = apptime.NewNullTime(t)
		}
	}
	return q
}

func recordToRoleQuota(rec *database.Record) RoleQuota {
	d := rec.Data
	return RoleQuota{
		ID:                rec.ID,
		RoleID:            stringVal(d["role_id"]),
		RoleName:          stringVal(d["role_name"]),
		MaxStorageBytes:   toInt64Val(d["max_storage_bytes"]),
		MaxBandwidthBytes: toInt64Val(d["max_bandwidth_bytes"]),
		MaxUploadSize:     toInt64Val(d["max_upload_size"]),
		MaxFilesCount:     toInt64Val(d["max_files_count"]),
		AllowedExtensions: stringVal(d["allowed_extensions"]),
		BlockedExtensions: stringVal(d["blocked_extensions"]),
		CreatedAt:         parseTime(d["created_at"]),
		UpdatedAt:         parseTime(d["updated_at"]),
	}
}

func recordToStorageShare(rec *database.Record) StorageShare {
	d := rec.Data
	share := StorageShare{
		ID:                rec.ID,
		ObjectID:          stringVal(d["object_id"]),
		PermissionLevel:   PermissionLevel(stringVal(d["permission_level"])),
		InheritToChildren: toBoolVal(d["inherit_to_children"]),
		IsPublic:          toBoolVal(d["is_public"]),
		CreatedBy:         stringVal(d["created_by"]),
		CreatedAt:         parseTime(d["created_at"]),
		UpdatedAt:         parseTime(d["updated_at"]),
	}
	if v := d["shared_with_user_id"]; v != nil {
		s := stringVal(v)
		if s != "" {
			share.SharedWithUserID = &s
		}
	}
	if v := d["shared_with_email"]; v != nil {
		s := stringVal(v)
		if s != "" {
			share.SharedWithEmail = &s
		}
	}
	if v := d["share_token"]; v != nil {
		s := stringVal(v)
		if s != "" {
			share.ShareToken = &s
		}
	}
	if v := d["expires_at"]; v != nil {
		s := stringVal(v)
		if s != "" {
			t := apptime.MustParse(s)
			share.ExpiresAt = apptime.NewNullTime(t)
		}
	}
	return share
}

func recordToUserQuotaOverride(rec *database.Record) UserQuotaOverride {
	d := rec.Data
	o := UserQuotaOverride{
		ID:        rec.ID,
		UserID:    stringVal(d["user_id"]),
		CreatedBy: stringVal(d["created_by"]),
		CreatedAt: parseTime(d["created_at"]),
		UpdatedAt: parseTime(d["updated_at"]),
	}
	if v := d["max_storage_bytes"]; v != nil {
		val := toInt64Val(v)
		o.MaxStorageBytes = &val
	}
	if v := d["max_bandwidth_bytes"]; v != nil {
		val := toInt64Val(v)
		o.MaxBandwidthBytes = &val
	}
	if v := d["max_upload_size"]; v != nil {
		val := toInt64Val(v)
		o.MaxUploadSize = &val
	}
	if v := d["max_files_count"]; v != nil {
		val := toInt64Val(v)
		o.MaxFilesCount = &val
	}
	if v := d["allowed_extensions"]; v != nil {
		s := stringVal(v)
		if s != "" {
			o.AllowedExtensions = &s
		}
	}
	if v := d["blocked_extensions"]; v != nil {
		s := stringVal(v)
		if s != "" {
			o.BlockedExtensions = &s
		}
	}
	if v := d["reason"]; v != nil {
		s := stringVal(v)
		if s != "" {
			o.Reason = &s
		}
	}
	if v := d["expires_at"]; v != nil {
		s := stringVal(v)
		if s != "" {
			t := apptime.MustParse(s)
			o.ExpiresAt = apptime.NewNullTime(t)
		}
	}
	return o
}
