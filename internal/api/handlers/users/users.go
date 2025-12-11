package users

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

type PaginatedUsersResponse struct {
	Data       []*auth.User `json:"data"`
	Total      int          `json:"total"`
	Page       int          `json:"page"`
	PageSize   int          `json:"pageSize"`
	TotalPages int          `json:"totalPages"`
}

func HandleGetUsers(userService *services.UserService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		page, _ := strconv.Atoi(r.URL.Query().Get("page"))
		if page < 1 {
			page = 1
		}

		pageSize, _ := strconv.Atoi(r.URL.Query().Get("page_size"))
		if pageSize < 1 || pageSize > 100 {
			pageSize = 20
		}

		users, total, err := userService.GetUsers(page, pageSize)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch users")
			return
		}

		totalPages := (total + pageSize - 1) / pageSize

		utils.JSONResponse(w, http.StatusOK, PaginatedUsersResponse{
			Data:       users,
			Total:      total,
			Page:       page,
			PageSize:   pageSize,
			TotalPages: totalPages,
		})
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
		if err := json.NewDecoder(r.Body).Decode(&updates); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
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
