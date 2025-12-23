package sha256

// HMAC computes HMAC-SHA256 of message with key.
//
// HMAC(K, m) = H((K' ⊕ opad) || H((K' ⊕ ipad) || m))
// where K' is the key padded to block size, ipad = 0x36, opad = 0x5c
//
// This implementation follows RFC 2104.
func HMAC(key, message []byte) []byte {
	// If key is longer than block size, hash it
	if len(key) > BlockSize {
		h := sum256(key)
		key = h[:]
	}

	// Pad key to block size
	paddedKey := make([]byte, BlockSize)
	copy(paddedKey, key)

	// Create inner and outer padded keys
	innerKey := make([]byte, BlockSize)
	outerKey := make([]byte, BlockSize)

	for i := 0; i < BlockSize; i++ {
		innerKey[i] = paddedKey[i] ^ 0x36
		outerKey[i] = paddedKey[i] ^ 0x5c
	}

	// Inner hash: H(innerKey || message)
	innerData := append(innerKey, message...)
	innerHash := sum256(innerData)

	// Outer hash: H(outerKey || innerHash)
	outerData := append(outerKey, innerHash[:]...)
	outerHash := sum256(outerData)

	return outerHash[:]
}
