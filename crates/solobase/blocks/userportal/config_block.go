package userportal

import (
	"encoding/json"

	"github.com/wafer-run/wafer-go"
)

const BlockName = "userportal-feature"

// UserPortalConfigBlock is a native wafer block for the userportal config endpoint.
type UserPortalConfigBlock struct {
	router *wafer.Router
	portal *UserPortalBlock
}

func NewUserPortalConfigBlock(portal *UserPortalBlock) *UserPortalConfigBlock {
	b := &UserPortalConfigBlock{portal: portal}
	b.router = wafer.NewRouter()
	b.router.Retrieve("/ext/userportal/config", b.handleGetConfig)
	return b
}

func (b *UserPortalConfigBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "User portal config",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
	}
}

func (b *UserPortalConfigBlock) Handle(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.router.Route(ctx, msg)
}

func (b *UserPortalConfigBlock) Lifecycle(_ wafer.Context, _ wafer.LifecycleEvent) error {
	return nil
}

func (b *UserPortalConfigBlock) handleGetConfig(_ wafer.Context, msg *wafer.Message) wafer.Result {
	data, err := json.Marshal(b.portal.config)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to serialize config")
	}
	return wafer.Respond(msg, 200, data, "application/json")
}
