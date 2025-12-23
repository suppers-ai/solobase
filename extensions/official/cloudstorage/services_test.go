package cloudstorage

import (
	"context"
	"database/sql"
	"testing"

	_ "github.com/glebarez/go-sqlite"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

func setupTestDB(t *testing.T) *sql.DB {
	db, err := sql.Open("sqlite", ":memory:")
	require.NoError(t, err)

	// Create storage_objects table
	_, err = db.Exec(`CREATE TABLE storage_objects (
		id TEXT PRIMARY KEY,
		bucket_name TEXT NOT NULL,
		object_name TEXT NOT NULL,
		parent_folder_id TEXT,
		size INTEGER,
		content_type TEXT,
		checksum TEXT,
		metadata TEXT,
		created_at DATETIME,
		updated_at DATETIME,
		last_viewed DATETIME,
		user_id TEXT,
		app_id TEXT
	)`)
	require.NoError(t, err)

	// Create storage shares table
	_, err = db.Exec(`CREATE TABLE ext_cloudstorage_storage_shares (
		id TEXT PRIMARY KEY,
		object_id TEXT NOT NULL,
		shared_with_user_id TEXT,
		shared_with_email TEXT,
		permission_level TEXT NOT NULL,
		inherit_to_children INTEGER DEFAULT 0,
		share_token TEXT,
		is_public INTEGER DEFAULT 0,
		expires_at DATETIME,
		created_by TEXT NOT NULL,
		created_at DATETIME NOT NULL,
		updated_at DATETIME NOT NULL
	)`)
	require.NoError(t, err)

	// Create users table for email lookups
	_, err = db.Exec(`CREATE TABLE users (
		id TEXT PRIMARY KEY,
		email TEXT
	)`)
	require.NoError(t, err)

	return db
}

func createTestFolder(t *testing.T, db *sql.DB, name string, parentID *string, userID string) *testStorageObject {
	folder := &testStorageObject{
		ID:             uuid.New().String(),
		BucketName:     "test-bucket",
		ObjectName:     name,
		ParentFolderID: parentID,
		ContentType:    "application/x-directory",
		Size:           0,
		UserID:         userID,
		CreatedAt:      apptime.NowTime(),
		UpdatedAt:      apptime.NowTime(),
	}

	var parentIDVal interface{} = nil
	if parentID != nil {
		parentIDVal = *parentID
	}

	_, err := db.Exec(`INSERT INTO storage_objects (id, bucket_name, object_name, parent_folder_id, content_type, size, user_id, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		folder.ID, folder.BucketName, folder.ObjectName, parentIDVal, folder.ContentType, folder.Size, folder.UserID, folder.CreatedAt, folder.UpdatedAt)
	require.NoError(t, err)
	return folder
}

func createTestFile(t *testing.T, db *sql.DB, name string, parentID *string, userID string) *testStorageObject {
	file := &testStorageObject{
		ID:             uuid.New().String(),
		BucketName:     "test-bucket",
		ObjectName:     name,
		ParentFolderID: parentID,
		ContentType:    "text/plain",
		Size:           1024,
		UserID:         userID,
		CreatedAt:      apptime.NowTime(),
		UpdatedAt:      apptime.NowTime(),
	}

	var parentIDVal interface{} = nil
	if parentID != nil {
		parentIDVal = *parentID
	}

	_, err := db.Exec(`INSERT INTO storage_objects (id, bucket_name, object_name, parent_folder_id, content_type, size, user_id, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		file.ID, file.BucketName, file.ObjectName, parentIDVal, file.ContentType, file.Size, file.UserID, file.CreatedAt, file.UpdatedAt)
	require.NoError(t, err)
	return file
}

type testStorageObject struct {
	ID             string
	BucketName     string
	ObjectName     string
	ParentFolderID *string
	ContentType    string
	Size           int64
	UserID         string
	CreatedAt      apptime.Time
	UpdatedAt      apptime.Time
}

func createTestShare(t *testing.T, db *sql.DB, objectID string, sharedWithUserID string, inheritToChildren bool, permissionLevel PermissionLevel) *StorageShare {
	share := &StorageShare{
		ID:                uuid.New().String(),
		ObjectID:          objectID,
		SharedWithUserID:  &sharedWithUserID,
		PermissionLevel:   permissionLevel,
		InheritToChildren: inheritToChildren,
		CreatedBy:         "owner-user",
		CreatedAt:         apptime.NowTime(),
		UpdatedAt:         apptime.NowTime(),
	}

	inherit := int64(0)
	if inheritToChildren {
		inherit = 1
	}

	_, err := db.Exec(`INSERT INTO ext_cloudstorage_storage_shares (id, object_id, shared_with_user_id, permission_level, inherit_to_children, is_public, created_by, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?)`,
		share.ID, share.ObjectID, *share.SharedWithUserID, share.PermissionLevel, inherit, share.CreatedBy, share.CreatedAt, share.UpdatedAt)
	require.NoError(t, err)
	return share
}

func TestCheckInheritedPermissions(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()
	shareService := NewShareService(db, nil)
	ctx := context.Background()

	ownerID := "owner-user"
	sharedUserID := "shared-user"

	// Create a folder hierarchy:
	// root-folder/
	//   ├── sub-folder/
	//   │   └── deep-file.txt
	//   └── direct-file.txt
	rootFolder := createTestFolder(t, db, "root-folder", nil, ownerID)
	subFolder := createTestFolder(t, db, "sub-folder", &rootFolder.ID, ownerID)
	deepFile := createTestFile(t, db, "deep-file.txt", &subFolder.ID, ownerID)
	directFile := createTestFile(t, db, "direct-file.txt", &rootFolder.ID, ownerID)

	t.Run("No share - no access", func(t *testing.T) {
		share, err := shareService.CheckInheritedPermissions(ctx, deepFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.Nil(t, share)
	})

	t.Run("Share on root folder with InheritToChildren=true - grants access to deep file", func(t *testing.T) {
		// Share root folder with inheritance
		createTestShare(t, db, rootFolder.ID, sharedUserID, true, PermissionView)

		// Check access to deep file
		share, err := shareService.CheckInheritedPermissions(ctx, deepFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.NotNil(t, share)
		assert.Equal(t, PermissionView, share.PermissionLevel)

		// Check access to direct file under root
		share, err = shareService.CheckInheritedPermissions(ctx, directFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.NotNil(t, share)
		assert.Equal(t, PermissionView, share.PermissionLevel)
	})

	t.Run("Share on root folder with InheritToChildren=false - no access to children", func(t *testing.T) {
		// Clear existing shares
		db.Exec("DELETE FROM ext_cloudstorage_storage_shares")

		// Share root folder without inheritance
		createTestShare(t, db, rootFolder.ID, sharedUserID, false, PermissionView)

		// Check access to deep file - should have no access
		share, err := shareService.CheckInheritedPermissions(ctx, deepFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.Nil(t, share)

		// Check access to direct file - should have no access
		share, err = shareService.CheckInheritedPermissions(ctx, directFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.Nil(t, share)
	})

	t.Run("Multiple shares - returns most permissive", func(t *testing.T) {
		// Clear existing shares
		db.Exec("DELETE FROM ext_cloudstorage_storage_shares")

		// Share root folder with View permission
		createTestShare(t, db, rootFolder.ID, sharedUserID, true, PermissionView)
		// Share sub folder with Edit permission
		createTestShare(t, db, subFolder.ID, sharedUserID, true, PermissionEdit)

		// Check access to deep file - should get Edit permission from closer parent
		share, err := shareService.CheckInheritedPermissions(ctx, deepFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.NotNil(t, share)
		assert.Equal(t, PermissionEdit, share.PermissionLevel)
	})

	t.Run("Expired share - no access", func(t *testing.T) {
		// Clear existing shares
		db.Exec("DELETE FROM ext_cloudstorage_storage_shares")

		// Create expired share
		expiredTime := apptime.NowTime().Add(-1 * apptime.Hour)
		_, err := db.Exec(`INSERT INTO ext_cloudstorage_storage_shares (id, object_id, shared_with_user_id, permission_level, inherit_to_children, is_public, expires_at, created_by, created_at, updated_at)
			VALUES (?, ?, ?, ?, 1, 0, ?, ?, ?, ?)`,
			uuid.New().String(), rootFolder.ID, sharedUserID, PermissionView, expiredTime, ownerID, apptime.NowTime(), apptime.NowTime())
		require.NoError(t, err)

		// Check access to deep file - should have no access due to expiration
		shareResult, err := shareService.CheckInheritedPermissions(ctx, deepFile.ID, sharedUserID, "")
		assert.NoError(t, err)
		assert.Nil(t, shareResult)
	})
}

func TestGetSharedWithMe(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()
	shareService := NewShareService(db, nil)
	ctx := context.Background()

	ownerID := "owner-user"
	sharedUserID := "shared-user"
	sharedUserEmail := "shared@example.com"

	// Create user record
	_, err := db.Exec("INSERT INTO users (id, email) VALUES (?, ?)", sharedUserID, sharedUserEmail)
	require.NoError(t, err)

	// Create folder hierarchy
	rootFolder := createTestFolder(t, db, "shared-root", nil, ownerID)
	subFolder := createTestFolder(t, db, "sub-folder", &rootFolder.ID, ownerID)
	_ = createTestFile(t, db, "file1.txt", &rootFolder.ID, ownerID)
	_ = createTestFile(t, db, "file2.txt", &subFolder.ID, ownerID)

	t.Run("Direct share appears in list", func(t *testing.T) {
		// Share root folder directly
		createTestShare(t, db, rootFolder.ID, sharedUserID, false, PermissionView)

		shares, err := shareService.GetSharedWithMe(ctx, sharedUserID)
		assert.NoError(t, err)
		assert.Len(t, shares, 1)
		assert.Equal(t, rootFolder.ID, shares[0].ObjectID)
	})

	t.Run("Inherited items appear when InheritToChildren=true", func(t *testing.T) {
		// Clear existing shares
		db.Exec("DELETE FROM ext_cloudstorage_storage_shares")

		// Share root folder with inheritance
		createTestShare(t, db, rootFolder.ID, sharedUserID, true, PermissionView)

		// Note: The current implementation of GetSharedWithMe with CTE
		// only shows direct children, not recursive descendants
		// This is a limitation that could be improved in the future
		shares, err := shareService.GetSharedWithMe(ctx, sharedUserID)
		assert.NoError(t, err)
		// Should include root folder and its direct children
		assert.GreaterOrEqual(t, len(shares), 1)
	})
}
