package admin

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/core/constants"
	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
)

const usersCollection = "auth_users"

func (b *AdminBlock) registerUsersRoutes() {
	b.router.Retrieve("/admin/users", b.handleGetUsers)
	b.router.Retrieve("/admin/users/{id}", b.handleGetUser)
	b.router.Update("/admin/users/{id}", b.handleUpdateUser)
	b.router.Delete("/admin/users/{id}", b.handleDeleteUser)
}

func (b *AdminBlock) handleGetUsers(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	page, pageSize, _ := msg.PaginationParams(constants.UsersPageSize)

	result, err := database.PaginatedList(context.Background(), db, usersCollection,
		page, pageSize,
		[]database.Filter{
			{Field: "deleted_at", Operator: database.OpIsNull},
		},
		[]database.SortField{{Field: "created_at", Desc: true}},
	)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to fetch users")
	}

	users := make([]map[string]any, 0, len(result.Records))
	for _, r := range result.Records {
		users = append(users, sanitizeUser(r.Data))
	}

	totalPages := (result.TotalCount + pageSize - 1) / pageSize
	return wafer.JSONRespond(msg, 200, map[string]any{
		"data":       users,
		"total":      result.TotalCount,
		"page":       page,
		"pageSize":   pageSize,
		"totalPages": totalPages,
	})
}

func (b *AdminBlock) handleGetUser(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.Var("id")
	record, err := db.Get(context.Background(), usersCollection, userID)
	if err != nil {
		if err == database.ErrNotFound {
			return wafer.Error(msg, 404, "not_found", "User not found")
		}
		return wafer.Error(msg, 500, "internal_error", "Failed to fetch user")
	}

	return wafer.JSONRespond(msg, 200, sanitizeUser(record.Data))
}

func (b *AdminBlock) handleUpdateUser(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.Var("id")

	var updates map[string]any
	if err := msg.Decode(&updates); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	for _, key := range []string{"id", "password", "created_at", "deleted_at"} {
		delete(updates, key)
	}

	record, err := db.Update(context.Background(), usersCollection, userID, updates)
	if err != nil {
		if err == database.ErrNotFound {
			return wafer.Error(msg, 404, "not_found", "User not found")
		}
		return wafer.Error(msg, 500, "internal_error", "Failed to update user")
	}

	return wafer.JSONRespond(msg, 200, sanitizeUser(record.Data))
}

func (b *AdminBlock) handleDeleteUser(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.Var("id")

	_, err := database.SoftDelete(context.Background(), db, usersCollection, userID)
	if err != nil {
		if err == database.ErrNotFound {
			return wafer.Error(msg, 404, "not_found", "User not found")
		}
		return wafer.Error(msg, 500, "internal_error", "Failed to delete user")
	}

	return wafer.JSONRespond(msg, 200, map[string]string{"message": "User deleted successfully"})
}

func sanitizeUser(data map[string]any) map[string]any {
	result := make(map[string]any, len(data))
	for k, v := range data {
		result[k] = v
	}
	for _, key := range []string{
		"password", "confirm_token", "confirm_selector",
		"recover_token", "recover_selector", "recover_token_exp",
		"totp_secret", "totp_secret_backup", "recovery_codes",
	} {
		delete(result, key)
	}
	if id, ok := data["id"]; ok {
		result["id"] = fmt.Sprintf("%v", id)
	}
	return result
}
