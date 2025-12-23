//go:build wasm

package logger

import (
	"context"
	"errors"
)

// FileLogger is a no-op stub in WASM builds
type FileLogger struct {
	fields map[string]interface{}
}

// NewFile returns an error in WASM builds as file logging is not supported
func NewFile(config Config) (*FileLogger, error) {
	return nil, errors.New("file logger not supported in WASM builds")
}

// Stub methods to satisfy Logger interface

func (l *FileLogger) Debug(ctx context.Context, message string, fields ...Field) {}
func (l *FileLogger) Info(ctx context.Context, message string, fields ...Field)  {}
func (l *FileLogger) Warn(ctx context.Context, message string, fields ...Field)  {}
func (l *FileLogger) Error(ctx context.Context, message string, fields ...Field) {}
func (l *FileLogger) Fatal(ctx context.Context, message string, fields ...Field) {}

func (l *FileLogger) With(fields ...Field) Logger {
	return l
}

func (l *FileLogger) WithContext(ctx context.Context) Logger {
	return l
}

func (l *FileLogger) LogRequest(ctx context.Context, req *RequestLog) error {
	return nil
}

func (l *FileLogger) GetLogs(ctx context.Context, filter LogFilter) ([]*Log, error) {
	return nil, nil
}

func (l *FileLogger) GetRequestLogs(ctx context.Context, filter RequestLogFilter) ([]*RequestLog, error) {
	return nil, nil
}

func (l *FileLogger) Flush() error {
	return nil
}

func (l *FileLogger) Close() error {
	return nil
}
