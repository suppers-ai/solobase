package admin

import (
	waffle "github.com/suppers-ai/waffle-go"
)

func (b *AdminBlock) registerWaffleRoutes() {
	b.router.Retrieve("/admin/waffle/blocks", b.handleListBlocks)
	b.router.Retrieve("/admin/waffle/flows", b.handleListFlows)
}

type blockListItem struct {
	Name         string              `json:"name"`
	Version      string              `json:"version"`
	Interface    string              `json:"interface"`
	Summary      string              `json:"summary"`
	InstanceMode string              `json:"instance_mode"`
	AllowedModes []string            `json:"allowed_modes"`
	AdminUI      *waffle.AdminUIInfo `json:"admin_ui,omitempty"`
}

func (b *AdminBlock) handleListBlocks(_ waffle.Context, msg *waffle.Message) waffle.Result {
	infos := b.runtime.Registry().List()

	items := make([]blockListItem, len(infos))
	for i, info := range infos {
		modes := make([]string, len(info.AllowedModes))
		for j, m := range info.AllowedModes {
			modes[j] = m.String()
		}
		items[i] = blockListItem{
			Name:         info.Name,
			Version:      info.Version,
			Interface:    info.Interface,
			Summary:      info.Summary,
			InstanceMode: info.InstanceMode.String(),
			AllowedModes: modes,
			AdminUI:      info.AdminUI,
		}
	}
	return waffle.JSONRespond(msg, 200, items)
}

func (b *AdminBlock) handleListFlows(_ waffle.Context, msg *waffle.Message) waffle.Result {
	defs := b.runtime.FlowDefs()
	return waffle.JSONRespond(msg, 200, defs)
}
