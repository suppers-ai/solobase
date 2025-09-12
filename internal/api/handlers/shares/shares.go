package shares

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/database"
	"gorm.io/gorm"
)

// StorageShare represents a share in the database
type StorageShare struct {
	ID                string     `gorm:"type:uuid;primaryKey" json:"id"`
	ObjectID          string     `gorm:"type:uuid;index;not null" json:"object_id"`
	ShareToken        string     `gorm:"type:varchar(255);unique;index" json:"share_token,omitempty"`
	CreatedBy         string     `gorm:"type:uuid;index;not null" json:"created_by"`
	SharedWithEmail   string     `gorm:"type:varchar(255);index" json:"shared_with_email,omitempty"`
	PermissionLevel   string     `gorm:"type:varchar(50);not null;default:'view'" json:"permission_level"`
	IsPublic          bool       `gorm:"default:false;index" json:"is_public"`
	InheritToChildren bool       `gorm:"default:false" json:"inherit_to_children"`
	ExpiresAt         *time.Time `gorm:"index" json:"expires_at,omitempty"`
	CreatedAt         time.Time  `gorm:"autoCreateTime" json:"created_at"`
	UpdatedAt         time.Time  `gorm:"autoUpdateTime" json:"updated_at"`
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
		// Get user ID from context
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			log.Printf("No user ID in context")
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		var shares []StorageShare
		if err := h.db.Where("created_by = ?", userID).Find(&shares).Error; err != nil {
			log.Printf("Error fetching shares: %v", err)
			http.Error(w, "Failed to fetch shares", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(shares)
	}
}

// HandleCreateShare creates a new share
func (h *SharesHandler) HandleCreateShare() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get user ID from context
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			log.Printf("No user ID in context")
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		var shareData StorageShare
		if err := json.NewDecoder(r.Body).Decode(&shareData); err != nil {
			log.Printf("Error decoding share data: %v", err)
			http.Error(w, "Invalid request body", http.StatusBadRequest)
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
			http.Error(w, "Failed to create share", http.StatusInternalServerError)
			return
		}

		log.Printf("Created share: %+v", shareData)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(shareData)
	}
}

// HandleGetShareByID returns a specific share
func (h *SharesHandler) HandleGetShareByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		shareID := vars["id"]

		// Get user ID from context
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			log.Printf("No user ID in context")
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		var share StorageShare
		if err := h.db.Where("id = ? AND created_by = ?", shareID, userID).First(&share).Error; err != nil {
			if err == gorm.ErrRecordNotFound {
				http.Error(w, "Share not found", http.StatusNotFound)
			} else {
				log.Printf("Error fetching share: %v", err)
				http.Error(w, "Failed to fetch share", http.StatusInternalServerError)
			}
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(share)
	}
}

// HandleDeleteShare deletes a share
func (h *SharesHandler) HandleDeleteShare() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		shareID := vars["id"]

		// Get user ID from context
		userID, ok := r.Context().Value("userID").(string)
		if !ok || userID == "" {
			log.Printf("No user ID in context")
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		// Delete the share
		result := h.db.Where("id = ? AND created_by = ?", shareID, userID).Delete(&StorageShare{})
		if result.Error != nil {
			log.Printf("Error deleting share: %v", result.Error)
			http.Error(w, "Failed to delete share", http.StatusInternalServerError)
			return
		}

		if result.RowsAffected == 0 {
			http.Error(w, "Share not found", http.StatusNotFound)
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
