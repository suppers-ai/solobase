//go:build !wasm

package sha256

import (
	"bytes"
	"crypto/hmac"
	stdsha256 "crypto/sha256"
	"testing"
)

// TestSum256MatchesStdlib verifies our implementation produces identical output to crypto/sha256
func TestSum256MatchesStdlib(t *testing.T) {
	testCases := [][]byte{
		{},
		[]byte("a"),
		[]byte("abc"),
		[]byte("message digest"),
		[]byte("abcdefghijklmnopqrstuvwxyz"),
		[]byte("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
		bytes.Repeat([]byte("a"), 1000),
		bytes.Repeat([]byte("ab"), 500),
		bytes.Repeat([]byte("\x00"), 100),
		bytes.Repeat([]byte("\xff"), 100),
		// Boundary cases around block size (64 bytes)
		bytes.Repeat([]byte("x"), 55),
		bytes.Repeat([]byte("x"), 56),
		bytes.Repeat([]byte("x"), 57),
		bytes.Repeat([]byte("x"), 63),
		bytes.Repeat([]byte("x"), 64),
		bytes.Repeat([]byte("x"), 65),
		bytes.Repeat([]byte("x"), 127),
		bytes.Repeat([]byte("x"), 128),
		bytes.Repeat([]byte("x"), 129),
	}

	for i, tc := range testCases {
		got := Sum256(tc)
		want := stdsha256.Sum256(tc)

		if got != want {
			t.Errorf("test case %d (len=%d): Sum256 mismatch\ngot:  %x\nwant: %x", i, len(tc), got, want)
		}
	}
}

// TestHMACMatchesStdlib verifies our HMAC implementation matches crypto/hmac
func TestHMACMatchesStdlib(t *testing.T) {
	keys := [][]byte{
		{},
		[]byte("key"),
		[]byte("secret"),
		bytes.Repeat([]byte("k"), 32),
		bytes.Repeat([]byte("k"), 64),
		bytes.Repeat([]byte("k"), 65),
		bytes.Repeat([]byte("k"), 128),
	}

	messages := [][]byte{
		{},
		[]byte(""),
		[]byte("message"),
		[]byte("The quick brown fox jumps over the lazy dog"),
		bytes.Repeat([]byte("m"), 1000),
	}

	for _, key := range keys {
		for _, msg := range messages {
			got := HMAC(key, msg)

			h := hmac.New(stdsha256.New, key)
			h.Write(msg)
			want := h.Sum(nil)

			if !bytes.Equal(got, want) {
				t.Errorf("HMAC mismatch for key(len=%d) msg(len=%d)\ngot:  %x\nwant: %x",
					len(key), len(msg), got, want)
			}
		}
	}
}

// Fuzz test against standard library
func FuzzSum256(f *testing.F) {
	// Add seed corpus
	f.Add([]byte{})
	f.Add([]byte("test"))
	f.Add([]byte("hello world"))
	f.Add(bytes.Repeat([]byte("a"), 100))

	f.Fuzz(func(t *testing.T, data []byte) {
		got := Sum256(data)
		want := stdsha256.Sum256(data)
		if got != want {
			t.Errorf("Sum256 mismatch for input len=%d", len(data))
		}
	})
}

func FuzzHMAC(f *testing.F) {
	f.Add([]byte("key"), []byte("message"))
	f.Add([]byte{}, []byte{})
	f.Add(bytes.Repeat([]byte("k"), 100), bytes.Repeat([]byte("m"), 100))

	f.Fuzz(func(t *testing.T, key, message []byte) {
		got := HMAC(key, message)

		h := hmac.New(stdsha256.New, key)
		h.Write(message)
		want := h.Sum(nil)

		if !bytes.Equal(got, want) {
			t.Errorf("HMAC mismatch for key(len=%d) msg(len=%d)", len(key), len(message))
		}
	})
}
