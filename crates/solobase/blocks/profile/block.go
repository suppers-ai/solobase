package profile

import (
	waffle "github.com/suppers-ai/waffle-go"
)

const BlockName = "profile-feature"

// ProfileBlock is a minimal native waffle block for the profile sections stub.
type ProfileBlock struct {
	router *waffle.Router
}

func NewProfileBlock() *ProfileBlock {
	b := &ProfileBlock{}
	b.router = waffle.NewRouter()
	b.router.Retrieve("/profile/sections", b.handleProfileSections)
	return b
}

func (b *ProfileBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Profile sections",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
	}
}

func (b *ProfileBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *ProfileBlock) Lifecycle(_ waffle.Context, _ waffle.LifecycleEvent) error {
	return nil
}

func (b *ProfileBlock) handleProfileSections(_ waffle.Context, msg *waffle.Message) waffle.Result {
	return waffle.JSONRespond(msg, 200, []any{})
}
