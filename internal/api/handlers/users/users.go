package users

import (
	"log"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

func HandleGetUsers(userService *services.UserService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		page, pageSize, _ := utils.GetPaginationParams(r, constants.UsersPageSize)

		users, total, err := userService.GetUsers(page, pageSize)
		if err != nil {
			log.Printf("GetUsers error: %v", err)
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch users")
			return
		}

		utils.SendPaginatedResponse(w, users, total, page, pageSize)
	}
}

func HandleGetUser(userService *services.UserService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		userID := vars["id"]

		user, err := userService.GetUserByID(userID)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "User not found")
			return
		}

		utils.JSONResponse(w, http.StatusOK, user)
	}
}

func HandleUpdateUser(userService *services.UserService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		userID := vars["id"]

		var updates map[string]interface{}
		if !utils.DecodeJSONBody(w, r, &updates) {
			return
		}

		user, err := userService.UpdateUser(userID, updates)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to update user")
			return
		}

		utils.JSONResponse(w, http.StatusOK, user)
	}
}

func HandleDeleteUser(userService *services.UserService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		userID := vars["id"]

		if err := userService.DeleteUser(userID); err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to delete user")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]string{"message": "User deleted successfully"})
	}
}
