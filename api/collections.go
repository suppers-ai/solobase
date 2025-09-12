package api

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
)

type Collection struct {
	ID           string           `json:"id"`
	Name         string           `json:"name"`
	Schema       CollectionSchema `json:"schema"`
	CreatedAt    time.Time        `json:"created_at"`
	UpdatedAt    time.Time        `json:"updated_at"`
	RecordsCount int              `json:"records_count"`
}

type CollectionSchema struct {
	Fields []CollectionField `json:"fields"`
}

type CollectionField struct {
	Name     string      `json:"name"`
	Type     string      `json:"type"`
	Required bool        `json:"required"`
	Unique   bool        `json:"unique,omitempty"`
	Default  interface{} `json:"default,omitempty"`
	Options  interface{} `json:"options,omitempty"`
}

type CreateCollectionRequest struct {
	Name   string           `json:"name"`
	Schema CollectionSchema `json:"schema"`
}

func HandleGetCollections(collectionService *services.CollectionService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		collections, err := collectionService.GetCollections()
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch collections")
			return
		}

		utils.JSONResponse(w, http.StatusOK, collections)
	}
}

func HandleGetCollection(collectionService *services.CollectionService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		collectionID := vars["id"]

		collection, err := collectionService.GetCollection(collectionID)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "Collection not found")
			return
		}

		utils.JSONResponse(w, http.StatusOK, collection)
	}
}

func HandleCreateCollection(collectionService *services.CollectionService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req CreateCollectionRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		collection, err := collectionService.CreateCollection(req.Name, req.Schema)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to create collection")
			return
		}

		utils.JSONResponse(w, http.StatusCreated, collection)
	}
}

func HandleUpdateCollection(collectionService *services.CollectionService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		collectionID := vars["id"]

		var updates map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&updates); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		collection, err := collectionService.UpdateCollection(collectionID, updates)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to update collection")
			return
		}

		utils.JSONResponse(w, http.StatusOK, collection)
	}
}

func HandleDeleteCollection(collectionService *services.CollectionService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		collectionID := vars["id"]

		if err := collectionService.DeleteCollection(collectionID); err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to delete collection")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "Collection deleted successfully"})
	}
}
