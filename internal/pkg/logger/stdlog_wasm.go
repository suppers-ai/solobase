//go:build wasm

package logger

import (
	"fmt"
	"os"
)

// StdLogPrintf uses fmt.Printf for WASM builds since log.Printf has issues in TinyGo
func StdLogPrintf(format string, v ...interface{}) {
	fmt.Printf(format+"\n", v...)
}

// StdLogPrintln uses fmt.Println for WASM builds
func StdLogPrintln(v ...interface{}) {
	fmt.Println(v...)
}

// StdLogFatal uses fmt.Println and os.Exit for WASM builds
func StdLogFatal(v ...interface{}) {
	fmt.Println(v...)
	os.Exit(1)
}
