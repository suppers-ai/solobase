// Package admin consolidates all admin-only feature blocks into a single block.
// It replaces: users, database, iam, logs, settings, wafer, and custom_tables.
package admin

import (
	"context"

	"github.com/suppers-ai/solobase/core/iam"
	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
)

const BlockName = "admin-feature"

// AdminBlock is a consolidated admin block handling users, database, IAM,
// logs, settings, wafer introspection, and custom tables.
type AdminBlock struct {
	router  *wafer.Router
	runtime *wafer.Wafer
	db      database.Service
}

// NewAdminBlock creates the admin block. The wafer runtime is obtained
// in Lifecycle(Init) via ctx.Service("wafer.runtime").
func NewAdminBlock() *AdminBlock {
	b := &AdminBlock{}
	b.router = wafer.NewRouter()
	b.registerUsersRoutes()
	b.registerDatabaseRoutes()
	b.registerIAMRoutes()
	b.registerLogsRoutes()
	b.registerSettingsRoutes()
	b.registerWaferRoutes()
	b.registerCustomTablesRoutes()
	return b
}

func (b *AdminBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Admin management",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
		AdminUI:      &wafer.AdminUIInfo{Path: "/admin/wafer", Icon: "settings", Title: "Admin"},
	}
}

func (b *AdminBlock) Handle(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.router.Route(ctx, msg)
}

func (b *AdminBlock) Lifecycle(ctx wafer.Context, evt wafer.LifecycleEvent) error {
	if evt.Type == wafer.Init {
		if wfl, ok := ctx.Service("wafer.runtime").(*wafer.Wafer); ok {
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
