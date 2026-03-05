package monitoring

import (
	"context"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/services/database"
)

const BlockName = "monitoring-feature"

const monitoringCollection = "sys_monitoring_snapshots"

// MonitoringBlock uses ctx.Services().Database for snapshot persistence.
type MonitoringBlock struct {
	router    *wafer.Router
	Collector *Collector
	persister *Persister
}

// NewMonitoringBlock creates a monitoring block that uses the database service.
func NewMonitoringBlock() *MonitoringBlock {
	collector := NewCollector()
	b := &MonitoringBlock{
		Collector: collector,
	}

	b.router = wafer.NewRouter()
	b.router.Retrieve("/admin/monitoring/live", b.handleLiveStats)
	b.router.Retrieve("/admin/monitoring/history", b.handleHistory)
	return b
}

func (b *MonitoringBlock) Info() wafer.BlockInfo {
	return wafer.BlockInfo{
		Name:         BlockName,
		Version:      "1.0.0",
		Interface:    "http.handler",
		Summary:      "Monitoring dashboard",
		InstanceMode: wafer.Singleton,
		AllowedModes: []wafer.InstanceMode{wafer.Singleton},
		AdminUI:      &wafer.AdminUIInfo{Path: "/admin", Icon: "layout-dashboard", Title: "Dashboard"},
	}
}

func (b *MonitoringBlock) Handle(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	return b.router.Route(ctx, msg)
}

func (b *MonitoringBlock) Lifecycle(ctx wafer.Context, evt wafer.LifecycleEvent) error {
	switch evt.Type {
	case wafer.Init:
		svc := ctx.Services()
		if svc == nil {
			return nil
		}
		db := svc.Database
		if db != nil {
			b.persister = NewPersister(b.Collector, db)
			b.persister.Start()
		}
	case wafer.Stop:
		if b.persister != nil {
			b.persister.Stop()
		}
	}
	return nil
}

func (b *MonitoringBlock) handleLiveStats(_ wafer.Context, msg *wafer.Message) wafer.Result {
	stats := b.Collector.ReadStats()
	return wafer.JSONRespond(msg, 200, stats)
}

func (b *MonitoringBlock) handleHistory(ctx wafer.Context, msg *wafer.Message) wafer.Result {
	db := ctx.Services().Database
	if db == nil {
		return wafer.JSONRespond(msg, 200, []any{})
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
		return wafer.Error(msg, 500, "internal_error", "Failed to fetch monitoring history")
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
			"perFlowJson":   r.Data["per_flow_json"],
			"perKindJson":   r.Data["per_kind_json"],
		})
	}

	return wafer.JSONRespond(msg, 200, snapshots)
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
	totalMessages, totalErrors, perBlockJSON, perFlowJSON, perKindJSON := p.collector.Snapshot()

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
		"per_flow_json":  perFlowJSON,
		"per_kind_json":  perKindJSON,
		"created_at":     now.Format(apptime.TimeFormat),
	}

	data["id"] = uuid.New().String()
	_, _ = p.db.Create(context.Background(), monitoringCollection, data)
	p.lastFlush = now
}
