package shares

import (
	"context"
	"database/sql"
	"log"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/utils"
)

// StorageShare represents a share in the database
type StorageShare struct {
	ID                string        `json:"id"`
	ObjectID          string        `json:"objectId"`
	ShareToken        string        `json:"shareToken,omitempty"`
	CreatedBy         string        `json:"createdBy"`
	SharedWithEmail   string        `json:"sharedWithEmail,omitempty"`
	PermissionLevel   string        `json:"permissionLevel"`
	IsPublic          bool          `json:"isPublic"`
	InheritToChildren bool          `json:"inheritToChildren"`
	ExpiresAt         *apptime.Time `json:"expiresAt,omitempty"`
	CreatedAt         apptime.Time  `json:"createdAt"`
	UpdatedAt         apptime.Time  `json:"updatedAt"`
}

// SharesHandler handles share-related API requests
type SharesHandler struct {
	db *sql.DB
}

// NewSharesHandler creates a new shares handler
func NewSharesHandler(sqlDB *sql.DB) *SharesHandler {
	return &SharesHandler{
		db: sqlDB,
	}
}

// HandleGetShares returns all shares created by the current user
func (h *SharesHandler) HandleGetShares() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		shares, err := h.getSharesByUser(r.Context(), userID)
		if err != nil {
			log.Printf("Error fetching shares: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch shares")
			return
		}

		utils.JSONResponse(w, http.StatusOK, shares)
	}
}

// HandleCreateShare creates a new share
func (h *SharesHandler) HandleCreateShare() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		var shareData StorageShare
		if !utils.DecodeJSONBody(w, r, &shareData) {
			return
		}

		// Set metadata
		shareData.ID = uuid.NewString()
		shareData.CreatedBy = userID
		now := apptime.NowTime()
		shareData.CreatedAt = now
		shareData.UpdatedAt = now

		// Generate share token if public
		if shareData.IsPublic {
			shareData.ShareToken = uuid.NewString()
		}

		// Default permission level
		if shareData.PermissionLevel == "" {
			shareData.PermissionLevel = "view"
		}

		// Create the share
		if err := h.createShare(r.Context(), &shareData); err != nil {
			log.Printf("Error creating share: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create share")
			return
		}

		log.Printf("Created share: %+v", shareData)
		utils.JSONResponse(w, http.StatusCreated, shareData)
	}
}

// HandleGetShareByID returns a specific share
func (h *SharesHandler) HandleGetShareByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		shareID := vars["id"]

		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		share, err := h.getShareByID(r.Context(), shareID, userID)
		if err != nil {
			if err == sql.ErrNoRows {
				utils.JSONError(w, http.StatusNotFound, "Share not found")
			} else {
				log.Printf("Error fetching share: %v", err)
				utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch share")
			}
			return
		}

		utils.JSONResponse(w, http.StatusOK, share)
	}
}

// HandleDeleteShare deletes a share
func (h *SharesHandler) HandleDeleteShare() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		shareID := vars["id"]

		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		// Delete the share
		affected, err := h.deleteShare(r.Context(), shareID, userID)
		if err != nil {
			log.Printf("Error deleting share: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to delete share")
			return
		}

		if affected == 0 {
			utils.JSONError(w, http.StatusNotFound, "Share not found")
			return
		}

		w.WriteHeader(http.StatusNoContent)
	}
}

// HandleShares routes share requests based on method
func (h *SharesHandler) HandleShares() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			h.HandleGetShares()(w, r)
		case http.MethodPost:
			h.HandleCreateShare()(w, r)
		case http.MethodOptions:
			w.WriteHeader(http.StatusOK)
		default:
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		}
	}
}

// HandleShareByID routes share requests by ID based on method
func (h *SharesHandler) HandleShareByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			h.HandleGetShareByID()(w, r)
		case http.MethodDelete:
			h.HandleDeleteShare()(w, r)
		case http.MethodOptions:
			w.WriteHeader(http.StatusOK)
		default:
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		}
	}
}

// Database operations using raw SQL

func (h *SharesHandler) getSharesByUser(ctx context.Context, userID string) ([]StorageShare, error) {
	query := `
		SELECT id, object_id, share_token, created_by, shared_with_email,
		       permission_level, is_public, inherit_to_children, expires_at,
		       created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE created_by = ?
		ORDER BY created_at DESC
	`

	rows, err := h.db.QueryContext(ctx, query, userID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var shares []StorageShare
	for rows.Next() {
		var share StorageShare
		var shareToken, sharedWithEmail sql.NullString
		var expiresAt apptime.NullTime
		var createdAt, updatedAt string
		var isPublic, inheritToChildren int64

		err := rows.Scan(
			&share.ID, &share.ObjectID, &shareToken, &share.CreatedBy,
			&sharedWithEmail, &share.PermissionLevel, &isPublic,
			&inheritToChildren, &expiresAt, &createdAt, &updatedAt,
		)
		if err != nil {
			return nil, err
		}

		if shareToken.Valid {
			share.ShareToken = shareToken.String
		}
		if sharedWithEmail.Valid {
			share.SharedWithEmail = sharedWithEmail.String
		}
		share.IsPublic = isPublic == 1
		share.InheritToChildren = inheritToChildren == 1
		if expiresAt.Valid {
			share.ExpiresAt = &expiresAt.Time
		}
		share.CreatedAt = apptime.MustParse(createdAt)
		share.UpdatedAt = apptime.MustParse(updatedAt)

		shares = append(shares, share)
	}

	if shares == nil {
		shares = []StorageShare{}
	}

	return shares, rows.Err()
}

func (h *SharesHandler) getShareByID(ctx context.Context, shareID, userID string) (*StorageShare, error) {
	query := `
		SELECT id, object_id, share_token, created_by, shared_with_email,
		       permission_level, is_public, inherit_to_children, expires_at,
		       created_at, updated_at
		FROM ext_cloudstorage_storage_shares
		WHERE id = ? AND created_by = ?
		LIMIT 1
	`

	var share StorageShare
	var shareToken, sharedWithEmail sql.NullString
	var expiresAt apptime.NullTime
	var createdAt, updatedAt string
	var isPublic, inheritToChildren int64

	err := h.db.QueryRowContext(ctx, query, shareID, userID).Scan(
		&share.ID, &share.ObjectID, &shareToken, &share.CreatedBy,
		&sharedWithEmail, &share.PermissionLevel, &isPublic,
		&inheritToChildren, &expiresAt, &createdAt, &updatedAt,
	)
	if err != nil {
		return nil, err
	}

	if shareToken.Valid {
		share.ShareToken = shareToken.String
	}
	if sharedWithEmail.Valid {
		share.SharedWithEmail = sharedWithEmail.String
	}
	share.IsPublic = isPublic == 1
	share.InheritToChildren = inheritToChildren == 1
	if expiresAt.Valid {
		share.ExpiresAt = &expiresAt.Time
	}
	share.CreatedAt = apptime.MustParse(createdAt)
	share.UpdatedAt = apptime.MustParse(updatedAt)

	return &share, nil
}

func (h *SharesHandler) createShare(ctx context.Context, share *StorageShare) error {
	query := `
		INSERT INTO ext_cloudstorage_storage_shares (
			id, object_id, share_token, created_by, shared_with_email,
			permission_level, is_public, inherit_to_children, expires_at,
			created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`

	var isPublic, inheritToChildren int64
	if share.IsPublic {
		isPublic = 1
	}
	if share.InheritToChildren {
		inheritToChildren = 1
	}

	var expiresAt interface{}
	if share.ExpiresAt != nil {
		expiresAt = apptime.Format(*share.ExpiresAt)
	}

	var shareToken interface{}
	if share.ShareToken != "" {
		shareToken = share.ShareToken
	}

	var sharedWithEmail interface{}
	if share.SharedWithEmail != "" {
		sharedWithEmail = share.SharedWithEmail
	}

	_, err := h.db.ExecContext(ctx, query,
		share.ID, share.ObjectID, shareToken, share.CreatedBy, sharedWithEmail,
		share.PermissionLevel, isPublic, inheritToChildren, expiresAt,
		apptime.Format(share.CreatedAt), apptime.Format(share.UpdatedAt),
	)
	return err
}

func (h *SharesHandler) deleteShare(ctx context.Context, shareID, userID string) (int64, error) {
	query := `DELETE FROM ext_cloudstorage_storage_shares WHERE id = ? AND created_by = ?`
	result, err := h.db.ExecContext(ctx, query, shareID, userID)
	if err != nil {
		return 0, err
	}
	return result.RowsAffected()
}
