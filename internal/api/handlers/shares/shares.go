package shares

import (
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/pkg/database"
	"github.com/suppers-ai/solobase/utils"
	"gorm.io/gorm"
)

// StorageShare represents a share in the database
type StorageShare struct {
	ID                string     `gorm:"type:uuid;primaryKey" json:"id"`
	ObjectID          string     `gorm:"type:uuid;index;not null" json:"objectId"`
	ShareToken        string     `gorm:"type:varchar(255);unique;index" json:"shareToken,omitempty"`
	CreatedBy         string     `gorm:"type:uuid;index;not null" json:"createdBy"`
	SharedWithEmail   string     `gorm:"type:varchar(255);index" json:"sharedWithEmail,omitempty"`
	PermissionLevel   string     `gorm:"type:varchar(50);not null;default:'view'" json:"permissionLevel"`
	IsPublic          bool       `gorm:"default:false;index" json:"isPublic"`
	InheritToChildren bool       `gorm:"default:false" json:"inheritToChildren"`
	ExpiresAt         *time.Time `gorm:"index" json:"expiresAt,omitempty"`
	CreatedAt         time.Time  `gorm:"autoCreateTime" json:"createdAt"`
	UpdatedAt         time.Time  `gorm:"autoUpdateTime" json:"updatedAt"`
}

// TableName specifies the table name
func (StorageShare) TableName() string {
	return "ext_cloudstorage_storage_shares"
}

// SharesHandler handles share-related API requests
type SharesHandler struct {
	db *gorm.DB
}

// NewSharesHandler creates a new shares handler
func NewSharesHandler(db *database.DB) *SharesHandler {
	return &SharesHandler{
		db: db.DB,
	}
}

// HandleGetShares returns all shares created by the current user
func (h *SharesHandler) HandleGetShares() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		userID, ok := utils.RequireUserID(w, r)
		if !ok {
			return
		}

		var shares []StorageShare
		if err := h.db.Where("created_by = ?", userID).Find(&shares).Error; err != nil {
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
		shareData.ID = uuid.New().String()
		shareData.CreatedBy = userID

		// Generate share token if public
		if shareData.IsPublic {
			shareData.ShareToken = uuid.New().String()
		}

		// Create the share
		if err := h.db.Create(&shareData).Error; err != nil {
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

		var share StorageShare
		if err := h.db.Where("id = ? AND created_by = ?", shareID, userID).First(&share).Error; err != nil {
			if err == gorm.ErrRecordNotFound {
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
		result := h.db.Where("id = ? AND created_by = ?", shareID, userID).Delete(&StorageShare{})
		if result.Error != nil {
			log.Printf("Error deleting share: %v", result.Error)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to delete share")
			return
		}

		if result.RowsAffected == 0 {
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
