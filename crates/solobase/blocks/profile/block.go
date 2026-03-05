package profile

import (
	wafer "github.com/wafer-run/wafer-go"
)

const BlockName = "profile-feature"

// ProfileBlock is a minimal native wafer block for the profile sections stub.
type ProfileBlock struct {
	router *wafer.Router
}

func NewProfileBlock() *ProfileBlock {
	b := &ProfileBlock{}
	b.router = wafer.NewRouter()
	b.router.Retrieve("/profile/sections", b.handleProfileSections)
	return b
}

func (b *ProfileBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Profile sections",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
	}
}

func (b *ProfileBlock) Handle(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.router.Route(ctx, msg)
}

func (b *ProfileBlock) Lifecycle(_ wafer.Context, _ wafer.LifecycleEvent) error {
	return nil
}

func (b *ProfileBlock) handleProfileSections(_ wafer.Context, msg *wafer.Message) wafer.Result {
	return wafer.JSONRespond(msg, 200, []any{})
}
