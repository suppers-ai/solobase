package monitoring

import (
	"context"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

const BlockName = "monitoring-feature"

const monitoringCollection = "sys_monitoring_snapshots"

// MonitoringBlock uses ctx.Services().Database for snapshot persistence.
type MonitoringBlock struct {
	router    *waffle.Router
	Collector *Collector
	persister *Persister
}

// NewMonitoringBlock creates a monitoring block that uses the database service.
func NewMonitoringBlock() *MonitoringBlock {
	collector := NewCollector()
	b := &MonitoringBlock{
		Collector: collector,
	}

	b.router = waffle.NewRouter()
	b.router.Retrieve("/admin/monitoring/live", b.handleLiveStats)
	b.router.Retrieve("/admin/monitoring/history", b.handleHistory)
	return b
}

func (b *MonitoringBlock) Info() waffle.BlockInfo {
	return waffle.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Monitoring dashboard",
		InstanceMode: waffle.Singleton,
		AllowedModes: []waffle.InstanceMode{waffle.Singleton},
		AdminUI:      &waffle.AdminUIInfo{Path: "/admin", Icon: "layout-dashboard", Title: "Dashboard"},
	}
}

func (b *MonitoringBlock) Handle(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	return b.router.Route(ctx, msg)
}

func (b *MonitoringBlock) Lifecycle(ctx waffle.Context, evt waffle.LifecycleEvent) error {
	switch evt.Type {
	case waffle.Init:
		svc := ctx.Services()
		if svc == nil {
			return nil
		}
		db := svc.Database
		if db != nil {
			b.persister = NewPersister(b.Collector, db)
			b.persister.Start()
		}
	case waffle.Stop:
		if b.persister != nil {
			b.persister.Stop()
		}
	}
	return nil
}

func (b *MonitoringBlock) handleLiveStats(_ waffle.Context, msg *waffle.Message) waffle.Result {
	stats := b.Collector.ReadStats()
	return waffle.JSONRespond(msg, 200, stats)
}

func (b *MonitoringBlock) handleHistory(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.JSONRespond(msg, 200, []any{})
	}

	timeRange := msg.Query("range")
	if timeRange == "" {
		timeRange = "24h"
	}

	startTime := calculateStartTime(timeRange)

	result, err := db.List(context.Background(), monitoringCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "period_start", Operator: database.OpGreaterEqual, Value: startTime.Format(apptime.TimeFormat)},
		},
		Sort:  []database.SortField{{Field: "period_start", Desc: true}},
		Limit: 1000,
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch monitoring history")
	}

	var snapshots []map[string]any
	for _, r := range result.Records {
		snapshots = append(snapshots, map[string]any{
			"id":            r.ID,
			"periodStart":   r.Data["period_start"],
			"periodEnd":     r.Data["period_end"],
			"totalMessages": r.Data["total_messages"],
			"totalErrors":   r.Data["total_errors"],
			"perBlockJson":  r.Data["per_block_json"],
			"perChainJson":  r.Data["per_chain_json"],
			"perKindJson":   r.Data["per_kind_json"],
		})
	}

	return waffle.JSONRespond(msg, 200, snapshots)
}

func calculateStartTime(timeRange string) apptime.Time {
	now := apptime.NowTime()
	switch timeRange {
	case "1h":
		return now.Add(-1 * apptime.Hour)
	case "6h":
		return now.Add(-6 * apptime.Hour)
	case "24h":
		return now.Add(-24 * apptime.Hour)
	case "7d":
		return now.Add(-7 * 24 * apptime.Hour)
	case "30d":
		return now.Add(-30 * 24 * apptime.Hour)
	default:
		return now.Add(-24 * apptime.Hour)
	}
}

// Persister periodically flushes collector snapshots to the database service.
type Persister struct {
	collector *Collector
	db        database.Service
	stop      chan struct{}
	lastFlush apptime.Time
}

// NewPersister creates a new persister that uses the database service.
func NewPersister(collector *Collector, db database.Service) *Persister {
	return &Persister{
		collector: collector,
		db:        db,
		stop:      make(chan struct{}),
		lastFlush: apptime.NowTime(),
	}
}

// Start begins periodic flushing (every 60 seconds).
func (p *Persister) Start() {
	go p.run()
}

// Stop signals the persister to stop.
func (p *Persister) Stop() {
	close(p.stop)
}

func (p *Persister) run() {
	ticker := apptime.NewTicker(60 * apptime.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			p.flush()
		case <-p.stop:
			p.flush()
			return
		}
	}
}

func (p *Persister) flush() {
	now := apptime.NowTime()
	totalMessages, totalErrors, perBlockJSON, perChainJSON, perKindJSON := p.collector.Snapshot()

	if totalMessages == 0 {
		p.lastFlush = now
		return
	}

	data := map[string]any{
		"period_start":   p.lastFlush.Format(apptime.TimeFormat),
		"period_end":     now.Format(apptime.TimeFormat),
		"total_messages": totalMessages,
		"total_errors":   totalErrors,
		"per_block_json": perBlockJSON,
		"per_chain_json": perChainJSON,
		"per_kind_json":  perKindJSON,
		"created_at":     now.Format(apptime.TimeFormat),
	}

	data["id"] = uuid.New().String()
	_, _ = p.db.Create(context.Background(), monitoringCollection, data)
	p.lastFlush = now
}
