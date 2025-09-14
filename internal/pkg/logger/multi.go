package logger

import (
	"context"
	"sync"
)

// MultiLogger implements Logger interface for multiple outputs
type MultiLogger struct {
	loggers []Logger
	mu      sync.RWMutex
}

// NewMulti creates a new multi logger with database support
func NewMulti(config Config, db interface{}) (*MultiLogger, error) {
	loggers := []Logger{}

	// Always add console logger
	consoleLogger, err := NewConsole(config)
	if err != nil {
		return nil, err
	}
	loggers = append(loggers, consoleLogger)

	// Add database logger if database provided
	if db != nil {
		dbLogger, err := NewDatabase(config, db)
		if err != nil {
			return nil, err
		}
		loggers = append(loggers, dbLogger)
	}

	// Add file logger if path specified
	if config.FilePath != "" {
		fileLogger, err := NewFile(config)
		if err != nil {
			return nil, err
		}
		loggers = append(loggers, fileLogger)
	}

	return &MultiLogger{
		loggers: loggers,
	}, nil
}

// NewMultiWithLoggers creates a multi logger with specific loggers
func NewMultiWithLoggers(loggers ...Logger) *MultiLogger {
	return &MultiLogger{
		loggers: loggers,
	}
}

// AddLogger adds a logger to the multi logger
func (l *MultiLogger) AddLogger(logger Logger) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.loggers = append(l.loggers, logger)
}

// RemoveLogger removes a logger from the multi logger
func (l *MultiLogger) RemoveLogger(logger Logger) {
	l.mu.Lock()
	defer l.mu.Unlock()

	newLoggers := []Logger{}
	for _, lg := range l.loggers {
		if lg != logger {
			newLoggers = append(newLoggers, lg)
		}
	}
	l.loggers = newLoggers
}

// Debug logs a debug message to all loggers
func (l *MultiLogger) Debug(ctx context.Context, message string, fields ...Field) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var wg sync.WaitGroup
	for _, logger := range l.loggers {
		wg.Add(1)
		go func(lg Logger) {
			defer wg.Done()
			lg.Debug(ctx, message, fields...)
		}(logger)
	}
	wg.Wait()
}

// Info logs an info message to all loggers
func (l *MultiLogger) Info(ctx context.Context, message string, fields ...Field) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var wg sync.WaitGroup
	for _, logger := range l.loggers {
		wg.Add(1)
		go func(lg Logger) {
			defer wg.Done()
			lg.Info(ctx, message, fields...)
		}(logger)
	}
	wg.Wait()
}

// Warn logs a warning message to all loggers
func (l *MultiLogger) Warn(ctx context.Context, message string, fields ...Field) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var wg sync.WaitGroup
	for _, logger := range l.loggers {
		wg.Add(1)
		go func(lg Logger) {
			defer wg.Done()
			lg.Warn(ctx, message, fields...)
		}(logger)
	}
	wg.Wait()
}

// Error logs an error message to all loggers
func (l *MultiLogger) Error(ctx context.Context, message string, fields ...Field) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var wg sync.WaitGroup
	for _, logger := range l.loggers {
		wg.Add(1)
		go func(lg Logger) {
			defer wg.Done()
			lg.Error(ctx, message, fields...)
		}(logger)
	}
	wg.Wait()
}

// Fatal logs a fatal message to all loggers and exits
func (l *MultiLogger) Fatal(ctx context.Context, message string, fields ...Field) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var wg sync.WaitGroup
	for _, logger := range l.loggers {
		wg.Add(1)
		go func(lg Logger) {
			defer wg.Done()
			lg.Fatal(ctx, message, fields...)
		}(logger)
	}
	wg.Wait()
}

// With creates a new logger with additional fields
func (l *MultiLogger) With(fields ...Field) Logger {
	l.mu.RLock()
	defer l.mu.RUnlock()

	newLoggers := []Logger{}
	for _, logger := range l.loggers {
		newLoggers = append(newLoggers, logger.With(fields...))
	}

	return &MultiLogger{
		loggers: newLoggers,
	}
}

// WithContext creates a new logger with context
func (l *MultiLogger) WithContext(ctx context.Context) Logger {
	l.mu.RLock()
	defer l.mu.RUnlock()

	newLoggers := []Logger{}
	for _, logger := range l.loggers {
		newLoggers = append(newLoggers, logger.WithContext(ctx))
	}

	return &MultiLogger{
		loggers: newLoggers,
	}
}

// LogRequest logs an HTTP request to all loggers
func (l *MultiLogger) LogRequest(ctx context.Context, req *RequestLog) error {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var firstErr error
	var wg sync.WaitGroup
	var errMu sync.Mutex

	for _, logger := range l.loggers {
		wg.Add(1)
		go func(lg Logger) {
			defer wg.Done()
			if err := lg.LogRequest(ctx, req); err != nil {
				errMu.Lock()
				if firstErr == nil {
					firstErr = err
				}
				errMu.Unlock()
			}
		}(logger)
	}
	wg.Wait()

	return firstErr
}

// GetLogs returns logs from the first logger that supports it
func (l *MultiLogger) GetLogs(ctx context.Context, filter LogFilter) ([]*Log, error) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	for _, logger := range l.loggers {
		logs, err := logger.GetLogs(ctx, filter)
		if err != nil {
			continue
		}
		if len(logs) > 0 || err == nil {
			return logs, nil
		}
	}

	return []*Log{}, nil
}

// GetRequestLogs returns request logs from the first logger that supports it
func (l *MultiLogger) GetRequestLogs(ctx context.Context, filter RequestLogFilter) ([]*RequestLog, error) {
	l.mu.RLock()
	defer l.mu.RUnlock()

	for _, logger := range l.loggers {
		logs, err := logger.GetRequestLogs(ctx, filter)
		if err != nil {
			continue
		}
		if len(logs) > 0 || err == nil {
			return logs, nil
		}
	}

	return []*RequestLog{}, nil
}

// Flush flushes all loggers
func (l *MultiLogger) Flush() error {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var firstErr error
	for _, logger := range l.loggers {
		if err := logger.Flush(); err != nil && firstErr == nil {
			firstErr = err
		}
	}

	return firstErr
}

// Close closes all loggers
func (l *MultiLogger) Close() error {
	l.mu.RLock()
	defer l.mu.RUnlock()

	var firstErr error
	for _, logger := range l.loggers {
		if err := logger.Close(); err != nil && firstErr == nil {
			firstErr = err
		}
	}

	return firstErr
}
