package system

import (
	"sort"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/waffle-go"
)

const BlockName = "system-feature"

// SystemBlock is a native waffle block for system infrastructure routes.
type SystemBlock struct {
	router        *waffle.Router
	waffleRuntime *waffle.Waffle
}

func NewSystemBlock() *SystemBlock {
	b := &SystemBlock{}
	b.router = waffle.NewRouter()
	// Public
	b.router.Retrieve("/health", b.handleHealth)
	b.router.Retrieve("/debug/time", b.handleDebugTime)
	// Protected
	b.router.Retrieve("/nav", b.handleGetNavItems)
	return b
}

func (b *SystemBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "System infrastructure routes",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
	}
}

func (b *SystemBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *SystemBlock) Lifecycle(ctx waffle.Context, evt waffle.LifecycleEvent) error {
	if evt.Type == waffle.Init {
		if wfl, ok := ctx.Service("waffle.runtime").(*waffle.Waffle); ok {
			b.waffleRuntime = wfl
		}
	}
	return nil
}

func (b *SystemBlock) handleHealth(_ waffle.Context, msg *waffle.Message) waffle.Result {
	return waffle.JSONRespond(msg, 200, map[string]any{
		"status":  "ok",
		"message": "API is running",
	})
}

func (b *SystemBlock) handleDebugTime(_ waffle.Context, msg *waffle.Message) waffle.Result {
	now := apptime.NowTime()
	return waffle.JSONRespond(msg, 200, map[string]any{
		"now":         now.String(),
		"unix":        now.Unix(),
		"unixNano":    now.UnixNano(),
		"rfc3339":     now.Format(apptime.TimeFormat),
		"isZero":      now.IsZero(),
		"year":        now.Year(),
		"startTime":   startTime.String(),
		"startIsZero": startTime.IsZero(),
	})
}

func (b *SystemBlock) handleGetNavItems(_ waffle.Context, msg *waffle.Message) waffle.Result {
	var items []NavItem
	if b.waffleRuntime != nil {
		for _, info := range b.waffleRuntime.Registry().List() {
			if info.AdminUI != nil {
				items = append(items, NavItem{
					Title: info.AdminUI.Title,
					Href:  info.AdminUI.Path,
					Icon:  info.AdminUI.Icon,
				})
			}
		}
	}
	// Add waffle admin items that aren't from blocks
	items = append(items,
		NavItem{Title: "Blocks", Href: "/admin/waffle#blocks", Icon: "package"},
		NavItem{Title: "Flows", Href: "/admin/waffle#flows", Icon: "git-branch"},
		NavItem{Title: "Logs", Href: "/admin/logs", Icon: "scroll-text"},
		NavItem{Title: "IAM", Href: "/admin/iam", Icon: "shield"},
	)
	// Sort: Dashboard first, then Blocks/Flows/Logs/IAM, then block admin UIs alphabetically
	sort.Slice(items, func(i, j int) bool {
		oi, oj := navOrder(items[i].Title), navOrder(items[j].Title)
		if oi != oj {
			return oi < oj
		}
		return items[i].Title < items[j].Title
	})
	// Add separator before the first block admin UI item (order group 2)
	for idx := range items {
		if navOrder(items[idx].Title) == 2 {
			items[idx].Separator = true
			break
		}
	}
	return waffle.JSONRespond(msg, 200, items)
}

func navOrder(title string) int {
	switch title {
	case "Dashboard":
		return 0
	case "Blocks", "Flows", "Logs", "IAM":
		return 1
	default:
		return 2
	}
}
