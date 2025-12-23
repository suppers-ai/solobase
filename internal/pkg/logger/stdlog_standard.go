//go:build !wasm

package logger

import "log"

// StdLogPrintf wraps standard library log.Printf
func StdLogPrintf(format string, v ...interface{}) {
	log.Printf(format, v...)
}

// StdLogPrintln wraps standard library log.Println
func StdLogPrintln(v ...interface{}) {
	log.Println(v...)
}

// StdLogFatal wraps standard library log.Fatal
func StdLogFatal(v ...interface{}) {
	log.Fatal(v...)
}
