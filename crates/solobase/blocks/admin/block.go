// Package admin consolidates all admin-only feature blocks into a single block.
// It replaces: users, database, iam, logs, settings, waffle, and custom_tables.
package admin

import (
	"context"

	"github.com/suppers-ai/solobase/core/iam"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

const BlockName = "admin-feature"

// AdminBlock is a consolidated admin block handling users, database, IAM,
// logs, settings, waffle introspection, and custom tables.
type AdminBlock struct {
	router  *waffle.Router
	runtime *waffle.Waffle
	db      database.Service
}

// NewAdminBlock creates the admin block. The waffle runtime is obtained
// in Lifecycle(Init) via ctx.Service("waffle.runtime").
func NewAdminBlock() *AdminBlock {
	b := &AdminBlock{}
	b.router = waffle.NewRouter()
	b.registerUsersRoutes()
	b.registerDatabaseRoutes()
	b.registerIAMRoutes()
	b.registerLogsRoutes()
	b.registerSettingsRoutes()
	b.registerWaffleRoutes()
	b.registerCustomTablesRoutes()
	return b
}

func (b *AdminBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Admin management",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
		AdminUI:      &waffle.AdminUIInfo{Path: "/admin/waffle", Icon: "settings", Title: "Admin"},
	}
}

func (b *AdminBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *AdminBlock) Lifecycle(ctx waffle.Context, evt waffle.LifecycleEvent) error {
	if evt.Type == waffle.Init {
		if wfl, ok := ctx.Service("waffle.runtime").(*waffle.Waffle); ok {
			b.runtime = wfl
		}
		if db := ctx.Services().Database; db != nil {
			b.db = db
			// Seed IAM default roles
			if err := iam.SeedDefaultRoles(context.Background(), db); err != nil {
				return err
			}
			// Initialize default settings
			if err := b.initializeDefaults(db); err != nil {
				return err
			}
		}
	}
	return nil
}
