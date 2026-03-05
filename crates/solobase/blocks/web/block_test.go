package web

import (
	"os"
	"path/filepath"
	"testing"

	wafer "github.com/wafer-run/wafer-go"
	"github.com/wafer-run/wafer-go/wafertest"
)

// setupTestSite creates a temp directory tree for testing:
//
//	index.html
//	about.html
//	css/style.css
//	assets/main.abc123.js
//	subdir/index.html
//	.env (dotfile)
func setupTestSite(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()

	files := map[string]string{
		"index.html":          "<html><body>home</body></html>",
		"about.html":          "<html><body>about</body></html>",
		"css/style.css":       "body { color: red; }",
		"assets/main.abc123.js": "console.log('hello');",
		"subdir/index.html":   "<html><body>subdir</body></html>",
		".env":                "SECRET=value",
	}

	for name, content := range files {
		p := filepath.Join(dir, name)
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return dir
}

func newTestBlock(t *testing.T, cfg WebConfig) *WebBlock {
	t.Helper()
	block := NewWebBlock(cfg)
	ctx := wafertest.NewContext(nil)
	wafertest.InitBlock(t, block, ctx)
	return block
}

func TestServeRoot(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	body := string(wafertest.ResponseBody(result))
	if body != "<html><body>home</body></html>" {
		t.Fatalf("unexpected body: %s", body)
	}
}

func TestServeStaticFile(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/about.html"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	body := string(wafertest.ResponseBody(result))
	if body != "<html><body>about</body></html>" {
		t.Fatalf("unexpected body: %s", body)
	}
	ct := result.Response.Meta[wafer.MetaRespContentType]
	if ct != "text/html; charset=utf-8" {
		t.Fatalf("expected text/html; charset=utf-8, got %s", ct)
	}
}

func TestServeNestedFile(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/css/style.css"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	body := string(wafertest.ResponseBody(result))
	if body != "body { color: red; }" {
		t.Fatalf("unexpected body: %s", body)
	}
	ct := result.Response.Meta[wafer.MetaRespContentType]
	if ct != "text/css; charset=utf-8" {
		t.Fatalf("expected text/css; charset=utf-8, got %s", ct)
	}
}

func TestServeHashedAsset(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/assets/main.abc123.js"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	cc := result.Response.Meta[wafer.MetaRespHeaderPrefix+"Cache-Control"]
	expected := "public, max-age=31536000, immutable"
	if cc != expected {
		t.Fatalf("expected %q, got %q", expected, cc)
	}
}

func TestPathTraversal(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/../../../etc/passwd"))
	if s := wafertest.Status(result); s != 404 {
		t.Fatalf("expected 404, got %d", s)
	}
}

func TestDotFileBlocked(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/.env"))
	if s := wafertest.Status(result); s != 404 {
		t.Fatalf("expected 404, got %d", s)
	}
}

func TestSPAFallback(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir, SPAMode: true})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/nonexistent/route"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	body := string(wafertest.ResponseBody(result))
	if body != "<html><body>home</body></html>" {
		t.Fatalf("expected index.html content, got: %s", body)
	}
}

func TestSPADisabled(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir, SPAMode: false})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/nonexistent/route"))
	if s := wafertest.Status(result); s != 404 {
		t.Fatalf("expected 404, got %d", s)
	}
}

func TestNonGetMethod(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Create("/", nil))
	if s := wafertest.Status(result); s != 405 {
		t.Fatalf("expected 405, got %d", s)
	}
}

func TestCacheHeadersHTML(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/about.html"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	cc := result.Response.Meta[wafer.MetaRespHeaderPrefix+"Cache-Control"]
	if cc != "no-cache" {
		t.Fatalf("expected no-cache, got %q", cc)
	}
}

func TestCacheHeadersNormal(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/css/style.css"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	cc := result.Response.Meta[wafer.MetaRespHeaderPrefix+"Cache-Control"]
	expected := "public, max-age=3600"
	if cc != expected {
		t.Fatalf("expected %q, got %q", expected, cc)
	}
}

func TestDirectoryServesIndex(t *testing.T) {
	dir := setupTestSite(t)
	block := newTestBlock(t, WebConfig{Dir: dir})
	ctx := wafertest.NewContext(nil)

	result := block.Handle(ctx, wafertest.Retrieve("/subdir/"))
	if s := wafertest.Status(result); s != 200 {
		t.Fatalf("expected 200, got %d", s)
	}
	body := string(wafertest.ResponseBody(result))
	if body != "<html><body>subdir</body></html>" {
		t.Fatalf("unexpected body: %s", body)
	}
}

func TestLifecycleInitMissingDir(t *testing.T) {
	block := NewWebBlock(WebConfig{Dir: "/nonexistent/path/that/does/not/exist"})
	ctx := wafertest.NewContext(nil)
	err := block.Lifecycle(ctx, wafer.LifecycleEvent{Type: wafer.Init})
	if err == nil {
		t.Fatal("expected error for missing dir")
	}
}

func TestLifecycleInitNotADir(t *testing.T) {
	// Create a temp file (not a directory)
	f, err := os.CreateTemp("", "webblock-test-*")
	if err != nil {
		t.Fatal(err)
	}
	f.Close()
	defer os.Remove(f.Name())

	block := NewWebBlock(WebConfig{Dir: f.Name()})
	ctx := wafertest.NewContext(nil)
	if err := block.Lifecycle(ctx, wafer.LifecycleEvent{Type: wafer.Init}); err == nil {
		t.Fatal("expected error for file path (not a directory)")
	}
}
