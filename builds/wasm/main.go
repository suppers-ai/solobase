//go:build wasm

// Solobase WASM Module
//
// This is the main entry point for WASM builds of Solobase.
// Exports an HTTP handler that the host calls for each request.
// The host provides database operations via wasmimport functions.
//
// Build:
//   tinygo build -target=wasip1 -gc=leaking -no-debug -tags wasm -o solobase.wasm .
//
// The host must implement the following imports:
//   - env.db_query(queryPtr, queryLen, argsPtr, argsLen) -> resultPtr
//   - env.db_exec(queryPtr, queryLen, argsPtr, argsLen) -> resultPtr
//   - env.db_begin() -> txID
//   - env.db_commit(txID) -> status
//   - env.db_rollback(txID) -> status
//   - env.get_config(keyPtr, keyLen) -> valuePtr

package main

import (
	"bytes"
	"embed"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
	"unsafe"

	"github.com/suppers-ai/solobase"
	"github.com/suppers-ai/solobase/builds/wasm/database"
	"github.com/suppers-ai/solobase/internal/env"
)

// Embed the frontend build for WASM
// Note: TinyGo requires embed in the main package
//
//go:embed frontend/build
var wasmUIFiles embed.FS

var (
	app     *solobase.App
	appOnce sync.Once
	initErr error
)

// HTTPRequest is the JSON request format from the host
type HTTPRequest struct {
	Method  string              `json:"method"`
	Path    string              `json:"path"`
	Headers map[string][]string `json:"headers"`
	Body    []byte              `json:"body,omitempty"`
}

// HTTPResponse is the JSON response format returned to the host
type HTTPResponse struct {
	Status  int                 `json:"status"`
	Headers map[string][]string `json:"headers"`
	Body    []byte              `json:"body,omitempty"`
}

// Host-provided config function
//
//go:wasmimport env get_config
func hostGetConfig(keyPtr, keyLen uint32) uint64

// getConfig retrieves a configuration value from the host
func getConfig(key, defaultVal string) string {
	keyBytes := []byte(key)
	if len(keyBytes) == 0 {
		return defaultVal
	}
	result := hostGetConfig(
		uint32(uintptr(unsafe.Pointer(&keyBytes[0]))),
		uint32(len(keyBytes)),
	)
	if result == 0 {
		return defaultVal
	}
	ptr := uint32(result >> 32)
	length := uint32(result & 0xFFFFFFFF)
	if ptr == 0 || length == 0 {
		return defaultVal
	}
	return string(readBytes(ptr, length))
}

// initApp initializes the Solobase application
func initApp() {
	fmt.Println("DEBUG: entering initApp")
	appOnce.Do(func() {
		fmt.Println("DEBUG: inside appOnce.Do")
		// Load config from host
		configKeys := []string{
			"JWT_SECRET",
			"DEFAULT_ADMIN_EMAIL",
			"DEFAULT_ADMIN_PASSWORD",
			"DATABASE_TYPE",
			"ENVIRONMENT",
			"PORT",
		}

		defaults := map[string]string{
			"JWT_SECRET":             "wasm-dev-secret-key-minimum-32-chars",
			"DEFAULT_ADMIN_EMAIL":    "",
			"DEFAULT_ADMIN_PASSWORD": "",
			"DATABASE_TYPE":          "sqlite",
			"ENVIRONMENT":            "production",
			"PORT":                   "8080",
		}

		for _, key := range configKeys {
			value := getConfig(key, defaults[key])
			env.SetEnv(key, value)
		}

		// Create host-provided database
		dbType := env.GetEnv("DATABASE_TYPE")
		if dbType == "" {
			dbType = "sqlite"
		}
		db := database.NewHostDBWithType(dbType)

		// Create Solobase app with the database and embedded UI
		app = solobase.NewWithOptions(&solobase.Options{
			Database:             db,
			DatabaseType:         dbType,
			DefaultAdminEmail:    env.GetEnv("DEFAULT_ADMIN_EMAIL"),
			DefaultAdminPassword: env.GetEnv("DEFAULT_ADMIN_PASSWORD"),
			JWTSecret:            env.GetEnv("JWT_SECRET"),
			Port:                 env.GetEnv("PORT"),
			ExternalUIFS:         &wasmUIFiles,
		})

		// Initialize and setup router (don't start server)
		if err := app.Initialize(); err != nil {
			initErr = err
			return
		}

		if err := app.SetupRouter(); err != nil {
			initErr = err
			return
		}
		fmt.Println("DEBUG: initApp completed successfully")
	})
	fmt.Println("DEBUG: exiting initApp")
}

// Memory management exports

//export solobase_alloc
func solobase_alloc(size uint32) uint32 {
	buf := make([]byte, size)
	return uint32(uintptr(unsafe.Pointer(&buf[0])))
}

//export solobase_free
func solobase_free(ptr uint32) {
	// No-op in Go - GC handles memory
}

// handle_request is the main entry point called by the host
//
//export handle_request
func handle_request(
	methodPtr, methodLen uint32,
	pathPtr, pathLen uint32,
	headersPtr, headersLen uint32,
	bodyPtr, bodyLen uint32,
) uint64 {
	fmt.Println("DEBUG: handle_request entry point")
	// Initialize app on first request
	initApp()
	fmt.Println("DEBUG: after initApp")

	// Check for initialization error
	if initErr != nil {
		return writeResponse(HTTPResponse{
			Status: http.StatusInternalServerError,
			Headers: map[string][]string{
				"Content-Type": {"application/json"},
			},
			Body: []byte(`{"error":"initialization_failed","message":"` + initErr.Error() + `"}`),
		})
	}

	// Parse request
	method := readString(methodPtr, methodLen)
	path := readString(pathPtr, pathLen)

	var headers map[string][]string
	if headersLen > 0 {
		headersJSON := readBytes(headersPtr, headersLen)
		json.Unmarshal(headersJSON, &headers)
	}
	if headers == nil {
		headers = make(map[string][]string)
	}

	var body []byte
	if bodyLen > 0 {
		body = readBytes(bodyPtr, bodyLen)
	}

	// Create HTTP request
	req, err := http.NewRequest(method, path, bytes.NewReader(body))
	if err != nil {
		return writeResponse(HTTPResponse{
			Status: http.StatusBadRequest,
			Headers: map[string][]string{
				"Content-Type": {"application/json"},
			},
			Body: []byte(`{"error":"invalid_request","message":"Failed to create request"}`),
		})
	}

	// Copy headers
	for key, values := range headers {
		for _, value := range values {
			req.Header.Add(key, value)
		}
	}

	// Create response writer
	w := newResponseWriter()

	// Route through Solobase
	fmt.Printf("DEBUG: WASM handle_request for path=%s\n", path)
	if app == nil {
		fmt.Println("DEBUG: app is nil!")
		return writeResponse(HTTPResponse{
			Status:  http.StatusInternalServerError,
			Headers: map[string][]string{"Content-Type": {"application/json"}},
			Body:    []byte(`{"error":"app_not_initialized"}`),
		})
	}
	router := app.Router()
	if router == nil {
		fmt.Println("DEBUG: router is nil!")
		return writeResponse(HTTPResponse{
			Status:  http.StatusInternalServerError,
			Headers: map[string][]string{"Content-Type": {"application/json"}},
			Body:    []byte(`{"error":"router_not_initialized"}`),
		})
	}
	fmt.Println("DEBUG: calling ServeHTTP")
	router.ServeHTTP(w, req)
	fmt.Println("DEBUG: ServeHTTP completed")

	// Build response
	return writeResponse(HTTPResponse{
		Status:  w.statusCode,
		Headers: w.headers,
		Body:    w.body.Bytes(),
	})
}

// handle_request_json is an alternative entry point that accepts JSON
//
//export handle_request_json
func handle_request_json(requestPtr, requestLen uint32) uint64 {
	// Initialize app on first request
	initApp()

	// Check for initialization error
	if initErr != nil {
		return writeResponse(HTTPResponse{
			Status: http.StatusInternalServerError,
			Headers: map[string][]string{
				"Content-Type": {"application/json"},
			},
			Body: []byte(`{"error":"initialization_failed","message":"` + initErr.Error() + `"}`),
		})
	}

	// Parse JSON request
	requestJSON := readBytes(requestPtr, requestLen)
	var httpReq HTTPRequest
	if err := json.Unmarshal(requestJSON, &httpReq); err != nil {
		return writeResponse(HTTPResponse{
			Status: http.StatusBadRequest,
			Headers: map[string][]string{
				"Content-Type": {"application/json"},
			},
			Body: []byte(`{"error":"invalid_json","message":"Failed to parse request JSON"}`),
		})
	}

	// Create HTTP request
	req, err := http.NewRequest(httpReq.Method, httpReq.Path, bytes.NewReader(httpReq.Body))
	if err != nil {
		return writeResponse(HTTPResponse{
			Status: http.StatusBadRequest,
			Headers: map[string][]string{
				"Content-Type": {"application/json"},
			},
			Body: []byte(`{"error":"invalid_request","message":"Failed to create request"}`),
		})
	}

	// Copy headers
	for key, values := range httpReq.Headers {
		for _, value := range values {
			req.Header.Add(key, value)
		}
	}

	// Create response writer
	w := newResponseWriter()

	// Route through Solobase
	app.Router().ServeHTTP(w, req)

	// Build response
	return writeResponse(HTTPResponse{
		Status:  w.statusCode,
		Headers: w.headers,
		Body:    w.body.Bytes(),
	})
}

// responseWriter captures HTTP response
type responseWriter struct {
	statusCode int
	headers    map[string][]string
	body       bytes.Buffer
}

func newResponseWriter() *responseWriter {
	return &responseWriter{
		statusCode: http.StatusOK,
		headers:    make(map[string][]string),
	}
}

func (w *responseWriter) Header() http.Header {
	return w.headers
}

func (w *responseWriter) Write(data []byte) (int, error) {
	return w.body.Write(data)
}

func (w *responseWriter) WriteHeader(statusCode int) {
	w.statusCode = statusCode
}

// Memory operations

func readString(ptr, len uint32) string {
	if ptr == 0 || len == 0 {
		return ""
	}
	return string(readBytes(ptr, len))
}

func readBytes(ptr, length uint32) []byte {
	if ptr == 0 || length == 0 {
		return nil
	}
	return unsafe.Slice((*byte)(unsafe.Pointer(uintptr(ptr))), length)
}

func writeBytes(data []byte) uint64 {
	if len(data) == 0 {
		return 0
	}
	ptr := uint32(uintptr(unsafe.Pointer(&data[0])))
	return (uint64(ptr) << 32) | uint64(len(data))
}

func writeResponse(resp HTTPResponse) uint64 {
	responseJSON, _ := json.Marshal(resp)
	return writeBytes(responseJSON)
}

func main() {
	// WASM: main is empty - host calls exported functions
}
