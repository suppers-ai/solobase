package metrics

import (
	"context"
	"fmt"
	"net/http"
	"sync"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// PrometheusCollector implements Collector using Prometheus
type PrometheusCollector struct {
	mu       sync.RWMutex
	registry *prometheus.Registry
	config   *Config

	// Metric stores
	counters   map[string]prometheus.Counter
	gauges     map[string]prometheus.Gauge
	histograms map[string]prometheus.Histogram
	summaries  map[string]prometheus.Summary

	// Vectors for labeled metrics
	counterVecs   map[string]*prometheus.CounterVec
	gaugeVecs     map[string]*prometheus.GaugeVec
	histogramVecs map[string]*prometheus.HistogramVec
	summaryVecs   map[string]*prometheus.SummaryVec
}

// NewPrometheusCollector creates a new Prometheus collector
func NewPrometheusCollector(config *Config) *PrometheusCollector {
	registry := prometheus.NewRegistry()

	// Register default collectors
	registry.MustRegister(prometheus.NewGoCollector())
	registry.MustRegister(prometheus.NewProcessCollector(prometheus.ProcessCollectorOpts{}))

	return &PrometheusCollector{
		registry:      registry,
		config:        config,
		counters:      make(map[string]prometheus.Counter),
		gauges:        make(map[string]prometheus.Gauge),
		histograms:    make(map[string]prometheus.Histogram),
		summaries:     make(map[string]prometheus.Summary),
		counterVecs:   make(map[string]*prometheus.CounterVec),
		gaugeVecs:     make(map[string]*prometheus.GaugeVec),
		histogramVecs: make(map[string]*prometheus.HistogramVec),
		summaryVecs:   make(map[string]*prometheus.SummaryVec),
	}
}

// Counter returns a counter metric
func (p *PrometheusCollector) Counter(name string, labels ...Label) Counter {
	p.mu.Lock()
	defer p.mu.Unlock()

	key := p.metricKey(name, labels)

	if len(labels) > 0 {
		// Use vector for labeled metrics
		vec, exists := p.counterVecs[name]
		if !exists {
			labelNames := p.getLabelNames(labels)
			vec = promauto.With(p.registry).NewCounterVec(
				prometheus.CounterOpts{
					Name: name,
					Help: fmt.Sprintf("Counter for %s", name),
				},
				labelNames,
			)
			p.counterVecs[name] = vec
		}

		labelValues := p.getLabelValues(labels)
		metric, _ := vec.GetMetricWithLabelValues(labelValues...)
		return &prometheusCounter{metric}
	}

	// Simple counter without labels
	counter, exists := p.counters[key]
	if !exists {
		counter = promauto.With(p.registry).NewCounter(prometheus.CounterOpts{
			Name: name,
			Help: fmt.Sprintf("Counter for %s", name),
		})
		p.counters[key] = counter
	}

	return &prometheusCounter{counter}
}

// Gauge returns a gauge metric
func (p *PrometheusCollector) Gauge(name string, labels ...Label) Gauge {
	p.mu.Lock()
	defer p.mu.Unlock()

	key := p.metricKey(name, labels)

	if len(labels) > 0 {
		// Use vector for labeled metrics
		vec, exists := p.gaugeVecs[name]
		if !exists {
			labelNames := p.getLabelNames(labels)
			vec = promauto.With(p.registry).NewGaugeVec(
				prometheus.GaugeOpts{
					Name: name,
					Help: fmt.Sprintf("Gauge for %s", name),
				},
				labelNames,
			)
			p.gaugeVecs[name] = vec
		}

		labelValues := p.getLabelValues(labels)
		metric, _ := vec.GetMetricWithLabelValues(labelValues...)
		return &prometheusGauge{metric}
	}

	// Simple gauge without labels
	gauge, exists := p.gauges[key]
	if !exists {
		gauge = promauto.With(p.registry).NewGauge(prometheus.GaugeOpts{
			Name: name,
			Help: fmt.Sprintf("Gauge for %s", name),
		})
		p.gauges[key] = gauge
	}

	return &prometheusGauge{gauge}
}

// Histogram returns a histogram metric
func (p *PrometheusCollector) Histogram(name string, buckets []float64, labels ...Label) Histogram {
	p.mu.Lock()
	defer p.mu.Unlock()

	key := p.metricKey(name, labels)

	if len(buckets) == 0 {
		buckets = prometheus.DefBuckets
	}

	if len(labels) > 0 {
		// Use vector for labeled metrics
		vec, exists := p.histogramVecs[name]
		if !exists {
			labelNames := p.getLabelNames(labels)
			vec = promauto.With(p.registry).NewHistogramVec(
				prometheus.HistogramOpts{
					Name:    name,
					Help:    fmt.Sprintf("Histogram for %s", name),
					Buckets: buckets,
				},
				labelNames,
			)
			p.histogramVecs[name] = vec
		}

		labelValues := p.getLabelValues(labels)
		metric, _ := vec.GetMetricWithLabelValues(labelValues...)
		return &prometheusHistogram{metric}
	}

	// Simple histogram without labels
	histogram, exists := p.histograms[key]
	if !exists {
		histogram = promauto.With(p.registry).NewHistogram(prometheus.HistogramOpts{
			Name:    name,
			Help:    fmt.Sprintf("Histogram for %s", name),
			Buckets: buckets,
		})
		p.histograms[key] = histogram
	}

	return &prometheusHistogram{histogram}
}

// Summary returns a summary metric
func (p *PrometheusCollector) Summary(name string, objectives map[float64]float64, labels ...Label) Summary {
	p.mu.Lock()
	defer p.mu.Unlock()

	key := p.metricKey(name, labels)

	if objectives == nil {
		objectives = map[float64]float64{
			0.5:  0.05,
			0.9:  0.01,
			0.99: 0.001,
		}
	}

	if len(labels) > 0 {
		// Use vector for labeled metrics
		vec, exists := p.summaryVecs[name]
		if !exists {
			labelNames := p.getLabelNames(labels)
			vec = promauto.With(p.registry).NewSummaryVec(
				prometheus.SummaryOpts{
					Name:       name,
					Help:       fmt.Sprintf("Summary for %s", name),
					Objectives: objectives,
				},
				labelNames,
			)
			p.summaryVecs[name] = vec
		}

		labelValues := p.getLabelValues(labels)
		metric, _ := vec.GetMetricWithLabelValues(labelValues...)
		return &prometheusSummary{metric}
	}

	// Simple summary without labels
	summary, exists := p.summaries[key]
	if !exists {
		summary = promauto.With(p.registry).NewSummary(prometheus.SummaryOpts{
			Name:       name,
			Help:       fmt.Sprintf("Summary for %s", name),
			Objectives: objectives,
		})
		p.summaries[key] = summary
	}

	return &prometheusSummary{summary}
}

// Collect collects metrics
func (p *PrometheusCollector) Collect(ctx context.Context) error {
	// Prometheus collects metrics automatically
	return nil
}

// Start starts the collector
func (p *PrometheusCollector) Start(ctx context.Context) error {
	// Prometheus doesn't need explicit start
	return nil
}

// Stop stops the collector
func (p *PrometheusCollector) Stop(ctx context.Context) error {
	// Prometheus doesn't need explicit stop
	return nil
}

// Reset resets all metrics
func (p *PrometheusCollector) Reset() {
	p.mu.Lock()
	defer p.mu.Unlock()

	// Create new registry
	p.registry = prometheus.NewRegistry()
	p.registry.MustRegister(prometheus.NewGoCollector())
	p.registry.MustRegister(prometheus.NewProcessCollector(prometheus.ProcessCollectorOpts{}))

	// Clear all metric stores
	p.counters = make(map[string]prometheus.Counter)
	p.gauges = make(map[string]prometheus.Gauge)
	p.histograms = make(map[string]prometheus.Histogram)
	p.summaries = make(map[string]prometheus.Summary)
	p.counterVecs = make(map[string]*prometheus.CounterVec)
	p.gaugeVecs = make(map[string]*prometheus.GaugeVec)
	p.histogramVecs = make(map[string]*prometheus.HistogramVec)
	p.summaryVecs = make(map[string]*prometheus.SummaryVec)
}

// Export exports metrics in Prometheus format
func (p *PrometheusCollector) Export() ([]byte, error) {
	// TODO: Implement text format export
	return nil, fmt.Errorf("not implemented")
}

// Handler returns HTTP handler for metrics endpoint
func (p *PrometheusCollector) Handler() interface{} {
	return promhttp.HandlerFor(p.registry, promhttp.HandlerOpts{
		EnableOpenMetrics: true,
	})
}

// Helper methods

func (p *PrometheusCollector) metricKey(name string, labels []Label) string {
	if len(labels) == 0 {
		return name
	}

	key := name
	for _, label := range labels {
		key += fmt.Sprintf("_%s_%s", label.Name, label.Value)
	}
	return key
}

func (p *PrometheusCollector) getLabelNames(labels []Label) []string {
	names := make([]string, len(labels))
	for i, label := range labels {
		names[i] = label.Name
	}
	return names
}

func (p *PrometheusCollector) getLabelValues(labels []Label) []string {
	values := make([]string, len(labels))
	for i, label := range labels {
		values[i] = label.Value
	}
	return values
}

// Wrapper types for Prometheus metrics

type prometheusCounter struct {
	prometheus.Counter
}

func (c *prometheusCounter) Inc() {
	c.Counter.Inc()
}

func (c *prometheusCounter) Add(v float64) {
	c.Counter.Add(v)
}

func (c *prometheusCounter) Get() float64 {
	// Prometheus doesn't provide direct Get, return 0
	return 0
}

type prometheusGauge struct {
	prometheus.Gauge
}

func (g *prometheusGauge) Set(v float64) {
	g.Gauge.Set(v)
}

func (g *prometheusGauge) Inc() {
	g.Gauge.Inc()
}

func (g *prometheusGauge) Dec() {
	g.Gauge.Dec()
}

func (g *prometheusGauge) Add(v float64) {
	g.Gauge.Add(v)
}

func (g *prometheusGauge) Sub(v float64) {
	g.Gauge.Sub(v)
}

func (g *prometheusGauge) Get() float64 {
	// Prometheus doesn't provide direct Get, return 0
	return 0
}

func (g *prometheusGauge) SetToCurrentTime() {
	g.Gauge.SetToCurrentTime()
}

type prometheusHistogram struct {
	prometheus.Histogram
}

func (h *prometheusHistogram) Observe(v float64) {
	h.Histogram.Observe(v)
}

func (h *prometheusHistogram) ObserveDuration(start time.Time) {
	h.Histogram.Observe(time.Since(start).Seconds())
}

type prometheusSummary struct {
	prometheus.Summary
}

func (s *prometheusSummary) Observe(v float64) {
	s.Summary.Observe(v)
}

func (s *prometheusSummary) ObserveDuration(start time.Time) {
	s.Summary.Observe(time.Since(start).Seconds())
}

// PrometheusHandler returns an HTTP handler for Prometheus metrics
func PrometheusHandler(collector *PrometheusCollector) http.Handler {
	return collector.Handler().(http.Handler)
}
