package admin

import (
	wafer "github.com/wafer-run/wafer-go"
)

func (b *AdminBlock) registerWaferRoutes() {
	b.router.Retrieve("/admin/wafer/blocks", b.handleListBlocks)
	b.router.Retrieve("/admin/wafer/flows", b.handleListFlows)
}

type blockListItem struct {
	Name         string              `json:"name"`
	Version      string              `json:"version"`
	Interface    string              `json:"interface"`
	Summary      string              `json:"summary"`
	InstanceMode string              `json:"instance_mode"`
	AllowedModes []string            `json:"allowed_modes"`
	AdminUI      *wafer.AdminUIInfo `json:"admin_ui,omitempty"`
}

func (b *AdminBlock) handleListBlocks(_ wafer.Context, msg *wafer.Message) wafer.Result {
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
	return wafer.JSONRespond(msg, 200, items)
}

func (b *AdminBlock) handleListFlows(_ wafer.Context, msg *wafer.Message) wafer.Result {
	defs := b.runtime.FlowDefs()
	return wafer.JSONRespond(msg, 200, defs)
}
