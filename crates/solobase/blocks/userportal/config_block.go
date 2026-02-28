package userportal

import (
	"encoding/json"

	"github.com/suppers-ai/waffle-go"
)

const BlockName = "userportal-feature"

// UserPortalConfigBlock is a native waffle block for the userportal config endpoint.
type UserPortalConfigBlock struct {
	router *waffle.Router
	portal *UserPortalBlock
}

func NewUserPortalConfigBlock(portal *UserPortalBlock) *UserPortalConfigBlock {
	b := &UserPortalConfigBlock{portal: portal}
	b.router = waffle.NewRouter()
	b.router.Retrieve("/ext/userportal/config", b.handleGetConfig)
	return b
}

func (b *UserPortalConfigBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "User portal config",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
	}
}

func (b *UserPortalConfigBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *UserPortalConfigBlock) Lifecycle(_ waffle.Context, _ waffle.LifecycleEvent) error {
	return nil
}

func (b *UserPortalConfigBlock) handleGetConfig(_ waffle.Context, msg *waffle.Message) waffle.Result {
	data, err := json.Marshal(b.portal.config)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to serialize config")
	}
	return waffle.Respond(msg, 200, data, "application/json")
}
