package web

import (
	"fmt"
	"regexp"
	"strings"
)

// hashPattern matches filenames with hash segments like main.a1b2c3d4.js or style-BkZ3xQ.css.
var hashPattern = regexp.MustCompile(`[.-][0-9a-zA-Z]{6,32}\.\w+$`)

// hashedAssetPrefixes are directories that typically contain fingerprinted assets.
var hashedAssetPrefixes = []string{
	"/assets/",
	"/_next/static/",
	"/static/js/",
	"/static/css/",
}

// cacheControl returns the appropriate Cache-Control header value.
func (b *WebBlock) cacheControl(reqPath, contentType string) string {
	// HTML files: always revalidate
	if strings.HasPrefix(contentType, "text/html") {
		return "no-cache"
	}

	// Hashed/fingerprinted assets: immutable
	if isHashedAsset(reqPath) {
		return fmt.Sprintf("public, max-age=%d, immutable", b.config.ImmutableMaxAge)
	}

	// Everything else: normal caching
	return fmt.Sprintf("public, max-age=%d", b.config.CacheMaxAge)
}

// isHashedAsset checks whether a request path looks like a fingerprinted asset.
func isHashedAsset(reqPath string) bool {
	// Check if the file is in a known hashed-asset directory
	inHashedDir := false
	for _, prefix := range hashedAssetPrefixes {
		if strings.HasPrefix(reqPath, prefix) {
			inHashedDir = true
			break
		}
	}
	if !inHashedDir {
		return false
	}

	// Check if the filename contains a hash segment
	return hashPattern.MatchString(reqPath)
}
