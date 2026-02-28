package routing

import (
	"testing"
)

func TestExtractSubdomain(t *testing.T) {
	tests := []struct {
		host string
		want string
	}{
		{"myapp.solobase.app", "myapp"},
		{"myapp.solobase.app:8080", "myapp"},
		{"solobase.app", ""},
		{"localhost", ""},
		{"localhost:8080", ""},
		{"sub.domain.solobase.app", "sub"},
		{"a.b.c.d", "a"},
	}
	for _, tt := range tests {
		got := extractSubdomain(tt.host)
		if got != tt.want {
			t.Errorf("extractSubdomain(%q) = %q, want %q", tt.host, got, tt.want)
		}
	}
}
