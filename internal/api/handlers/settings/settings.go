package settings

import (
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

func HandleGetSettings(settingsService *services.SettingsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		settings, err := settingsService.GetSettings()
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch settings")
			return
		}

		utils.JSONResponse(w, http.StatusOK, settings)
	}
}

func HandleUpdateSettings(settingsService *services.SettingsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !utils.IsAdmin(r) {
			utils.JSONError(w, http.StatusForbidden, "Insufficient permissions")
			return
		}

		var updates map[string]interface{}
		if !utils.DecodeJSONBody(w, r, &updates) {
			return
		}

		settings, err := settingsService.UpdateSettings(updates)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to update settings")
			return
		}

		utils.JSONResponse(w, http.StatusOK, settings)
	}
}

func HandleResetSettings(settingsService *services.SettingsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !utils.IsAdmin(r) {
			utils.JSONError(w, http.StatusForbidden, "Insufficient permissions")
			return
		}

		if err := settingsService.ResetToDefaults(); err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to reset settings")
			return
		}

		settings, err := settingsService.GetSettings()
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch settings after reset")
			return
		}

		utils.JSONResponse(w, http.StatusOK, settings)
	}
}

// HandleGetSetting gets a single setting by key
func HandleGetSetting(settingsService *services.SettingsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		key := vars["key"]

		value, err := settingsService.GetSetting(key)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "Setting not found")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"key":   key,
			"value": value,
		})
	}
}

// HandleSetSetting creates or updates a single setting
func HandleSetSetting(settingsService *services.SettingsService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !utils.IsAdmin(r) {
			utils.JSONError(w, http.StatusForbidden, "Insufficient permissions")
			return
		}

		var req struct {
			Key   string      `json:"key"`
			Value interface{} `json:"value"`
			Type  string      `json:"type,omitempty"`
		}

		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		if req.Key == "" {
			utils.JSONError(w, http.StatusBadRequest, "Setting key is required")
			return
		}

		if err := settingsService.SetSetting(req.Key, req.Value); err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to update setting")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"success": true,
			"key":     req.Key,
			"value":   req.Value,
		})
	}
}
