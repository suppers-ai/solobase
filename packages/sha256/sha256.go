// Package sha256 provides a pure-Go implementation of SHA-256.
//
// This implementation is designed for environments where crypto/sha256
// is not available, such as TinyGo WASM builds where crypto/internal/fips140
// is not supported.
//
// The implementation follows FIPS 180-4 and RFC 6234.
package sha256

import (
	"encoding/binary"
)

// Size is the size, in bytes, of a SHA-256 checksum.
const Size = 32

// BlockSize is the block size, in bytes, of the SHA-256 hash function.
const BlockSize = 64

// Sum256 returns the SHA-256 checksum of the data.
func Sum256(data []byte) [Size]byte {
	return sum256(data)
}

// Sum returns the SHA-256 checksum of the data as a byte slice.
func Sum(data []byte) []byte {
	h := sum256(data)
	return h[:]
}

// initial hash values (first 32 bits of the fractional parts of the square roots of the first 8 primes)
var h0 = [8]uint32{
	0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
	0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
}

// round constants (first 32 bits of the fractional parts of the cube roots of the first 64 primes)
var k = [64]uint32{
	0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
	0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
	0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
	0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
	0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
	0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
	0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
	0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
}

// sum256 computes SHA-256 hash
func sum256(data []byte) [Size]byte {
	// Initialize hash values
	h := h0

	// Pre-processing: adding padding bits
	msgLen := uint64(len(data))
	// Pad to 448 mod 512 bits (56 mod 64 bytes), plus 8 bytes for length
	padLen := (55 - int(msgLen)) % 64
	if padLen < 0 {
		padLen += 64
	}

	// Create padded message
	padded := make([]byte, len(data)+1+padLen+8)
	copy(padded, data)
	padded[len(data)] = 0x80 // Append bit '1'
	// Append original length in bits as big-endian 64-bit integer
	binary.BigEndian.PutUint64(padded[len(padded)-8:], msgLen*8)

	// Process each 64-byte chunk
	var w [64]uint32
	for i := 0; i < len(padded); i += BlockSize {
		// Break chunk into sixteen 32-bit big-endian words
		for j := 0; j < 16; j++ {
			w[j] = binary.BigEndian.Uint32(padded[i+j*4:])
		}

		// Extend the sixteen 32-bit words into sixty-four 32-bit words
		for j := 16; j < 64; j++ {
			s0 := rotr(w[j-15], 7) ^ rotr(w[j-15], 18) ^ (w[j-15] >> 3)
			s1 := rotr(w[j-2], 17) ^ rotr(w[j-2], 19) ^ (w[j-2] >> 10)
			w[j] = w[j-16] + s0 + w[j-7] + s1
		}

		// Initialize working variables
		a, b, c, d, e, f, g, hh := h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]

		// Main compression loop
		for j := 0; j < 64; j++ {
			S1 := rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25)
			ch := (e & f) ^ (^e & g)
			temp1 := hh + S1 + ch + k[j] + w[j]
			S0 := rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22)
			maj := (a & b) ^ (a & c) ^ (b & c)
			temp2 := S0 + maj

			hh = g
			g = f
			f = e
			e = d + temp1
			d = c
			c = b
			b = a
			a = temp1 + temp2
		}

		// Add compressed chunk to current hash value
		h[0] += a
		h[1] += b
		h[2] += c
		h[3] += d
		h[4] += e
		h[5] += f
		h[6] += g
		h[7] += hh
	}

	// Produce the final hash value (big-endian)
	var result [Size]byte
	for i := 0; i < 8; i++ {
		binary.BigEndian.PutUint32(result[i*4:], h[i])
	}

	return result
}

// rotr performs a right rotation on a 32-bit value
func rotr(x uint32, n uint) uint32 {
	return (x >> n) | (x << (32 - n))
}
