package admin

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/core/iam"
	"github.com/suppers-ai/solobase/core/constants"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

func (b *AdminBlock) registerIAMRoutes() {
	b.router.Retrieve("/admin/iam/roles", b.handleGetRoles)
	b.router.Create("/admin/iam/roles", b.handleCreateRole)
	b.router.Delete("/admin/iam/roles/{id}", b.handleDeleteRole)
	b.router.Retrieve("/admin/iam/users", b.handleGetUsersWithRoles)
	b.router.Retrieve("/admin/iam/users/{userId}/roles", b.handleGetUserRoles)
	b.router.Create("/admin/iam/users/{userId}/roles", b.handleAssignRole)
	b.router.Delete("/admin/iam/users/{userId}/roles/{roleName}", b.handleRemoveRole)
}

func (b *AdminBlock) handleGetRoles(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	records, err := database.ListAll(context.Background(), db, "iam_roles")
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}

	roles := make([]iam.Role, 0, len(records))
	for _, r := range records {
		roles = append(roles, iam.Role{
			ID:          r.ID,
			Name:        strVal(r.Data["name"]),
			DisplayName: strVal(r.Data["display_name"]),
			Description: strVal(r.Data["description"]),
			IsSystem:    boolVal(r.Data["is_system"]),
		})
	}
	return waffle.JSONRespond(msg, 200, roles)
}

func (b *AdminBlock) handleCreateRole(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	var role iam.Role
	if err := msg.Decode(&role); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	record, err := db.Create(context.Background(), "iam_roles", map[string]any{
		"name":         role.Name,
		"display_name": role.DisplayName,
		"description":  role.Description,
		"is_system":    false,
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}

	role.ID = record.ID
	role.IsSystem = false
	return waffle.JSONRespond(msg, 201, role)
}

func (b *AdminBlock) handleDeleteRole(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	roleID := msg.Var("id")

	record, err := db.Get(context.Background(), "iam_roles", roleID)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", "Role not found")
	}
	if boolVal(record.Data["is_system"]) {
		return waffle.Error(msg, 400, "bad_request", "Cannot delete system role")
	}

	roleName := strVal(record.Data["name"])
	if roleName != "" {
		if err := database.DeleteByField(context.Background(), db, "iam_user_roles", "role_name", roleName); err != nil {
			return waffle.Error(msg, 500, "internal_error", "Failed to remove role assignments")
		}
	}

	if err := db.Delete(context.Background(), "iam_roles", roleID); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}

func (b *AdminBlock) handleGetUsersWithRoles(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	result, err := database.PaginatedList(context.Background(), db, usersCollection,
		1, constants.UsersPageSize,
		[]database.Filter{
			{Field: "deleted_at", Operator: database.OpIsNull},
		},
		[]database.SortField{{Field: "created_at", Desc: true}},
	)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch users")
	}

	bgCtx := context.Background()
	var users []map[string]any
	for _, r := range result.Records {
		userID := fmt.Sprintf("%v", r.Data["id"])
		if userID == "" || userID == "<nil>" {
			userID = r.ID
		}

		roles, _ := iam.GetUserRoles(bgCtx, db, userID)

		roleDetails := make([]map[string]any, 0)
		for _, roleName := range roles {
			displayName := roleName
			roleRecord, err := database.GetByField(bgCtx, db, "iam_roles", "name", roleName)
			if err == nil {
				displayName = strVal(roleRecord.Data["display_name"])
				if displayName == "" {
					displayName = roleName
				}
			}
			roleDetails = append(roleDetails, map[string]any{
				"name":        roleName,
				"displayName": displayName,
			})
		}

		userMap := map[string]any{
			"id":        userID,
			"email":     r.Data["email"],
			"firstName": r.Data["first_name"],
			"lastName":  r.Data["last_name"],
			"createdAt": r.Data["created_at"],
			"roles":     roleDetails,
		}
		if v := r.Data["last_login"]; v != nil && v != "" {
			userMap["lastLogin"] = v
		}
		users = append(users, userMap)
	}
	return waffle.JSONRespond(msg, 200, users)
}

func (b *AdminBlock) handleGetUserRoles(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.Var("userId")
	roles, err := iam.GetUserRoles(context.Background(), db, userID)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 200, roles)
}

func (b *AdminBlock) handleAssignRole(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.Var("userId")

	var body struct {
		Role string `json:"role"`
	}
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	grantedBy := msg.UserID()
	if grantedBy == "" {
		grantedBy = "system"
	}

	if err := iam.AssignRole(context.Background(), db, userID, body.Role, grantedBy); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.JSONRespond(msg, 201, map[string]string{"status": "assigned"})
}

func (b *AdminBlock) handleRemoveRole(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	userID := msg.Var("userId")
	roleName := msg.Var("roleName")

	if err := iam.RemoveRole(context.Background(), db, userID, roleName); err != nil {
		return waffle.Error(msg, 500, "internal_error", err.Error())
	}
	return waffle.Respond(msg, 204, nil, "")
}
