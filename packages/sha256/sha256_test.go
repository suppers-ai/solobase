package sha256

import (
	"bytes"
	"encoding/hex"
	"strings"
	"testing"
)

// NIST FIPS 180-4 test vectors for SHA-256
// https://csrc.nist.gov/CSRC/media/Projects/Cryptographic-Standards-and-Guidelines/documents/examples/SHA256.pdf
var sha256Tests = []struct {
	name   string
	input  string
	expect string
}{
	{
		name:   "empty",
		input:  "",
		expect: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
	},
	{
		name:   "NIST one-block message (abc)",
		input:  "abc",
		expect: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
	},
	{
		name:   "NIST two-block message",
		input:  "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
		expect: "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
	},
	{
		name:   "NIST long message (896 bits)",
		input:  "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
		expect: "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
	},
	{
		name:   "single character",
		input:  "a",
		expect: "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
	},
	{
		name:   "The quick brown fox",
		input:  "The quick brown fox jumps over the lazy dog",
		expect: "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
	},
	{
		name:   "The quick brown fox with period",
		input:  "The quick brown fox jumps over the lazy dog.",
		expect: "ef537f25c895bfa782526529a9b63d97aa631564d5d789c2b765448c8635fb6c",
	},
	{
		name:   "55 bytes (padding boundary - 1)",
		input:  "1234567890123456789012345678901234567890123456789012345",
		expect: "03c3a70e99ed5eeccd80f73771fcf1ece643d939d9ecc76f25544b0233f708e9",
	},
	{
		name:   "56 bytes (exactly padding boundary)",
		input:  "12345678901234567890123456789012345678901234567890123456",
		expect: "0be66ce72c2467e793202906000672306661791622e0ca9adf4a8955b2ed189c",
	},
	{
		name:   "57 bytes (padding boundary + 1)",
		input:  "123456789012345678901234567890123456789012345678901234567",
		expect: "1482865e0eaedf2e10728c051c3b7457f4a416d0a2c70f2f198b4d54b223a0a0",
	},
	{
		name:   "63 bytes (block size - 1)",
		input:  "123456789012345678901234567890123456789012345678901234567890123",
		expect: "b97f6a278ef6a159ba660dc99fc5426ae3c1e4e08c471827d660bf36cfb236e7",
	},
	{
		name:   "64 bytes (exactly one block)",
		input:  "1234567890123456789012345678901234567890123456789012345678901234",
		expect: "676491965ed3ec50cb7a63ee96315480a95c54426b0b72bca8a0d4ad1285ad55",
	},
	{
		name:   "65 bytes (one block + 1)",
		input:  "12345678901234567890123456789012345678901234567890123456789012345",
		expect: "71fbbf9bcb342cdc7768b7d494089e947ac411548fd9fd6f67bb7a207928027d",
	},
	{
		name:   "128 bytes (exactly two blocks)",
		input:  "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678",
		expect: "fb8d8391fc40575dca6d3c363bc46a3f64ebf484fba9187cdf62aee6cbce6c1f",
	},
	{
		name:   "binary zeros",
		input:  "\x00\x00\x00\x00",
		expect: "df3f619804a92fdb4057192dc43dd748ea778adc52bc498ce80524c014b81119",
	},
	{
		name:   "all 0xFF bytes",
		input:  "\xff\xff\xff\xff",
		expect: "ad95131bc0b799c0b1af477fb14fcf26a6a9f76079e48bf090acb7e8367bfd0e",
	},
}

func TestSum256(t *testing.T) {
	for _, tt := range sha256Tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Sum256([]byte(tt.input))
			gotHex := hex.EncodeToString(got[:])
			if gotHex != tt.expect {
				t.Errorf("Sum256(%q) = %s, want %s", tt.input, gotHex, tt.expect)
			}
		})
	}
}

func TestSum(t *testing.T) {
	for _, tt := range sha256Tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Sum([]byte(tt.input))
			gotHex := hex.EncodeToString(got)
			if gotHex != tt.expect {
				t.Errorf("Sum(%q) = %s, want %s", tt.input, gotHex, tt.expect)
			}
		})
	}
}

// Test with 1 million 'a' characters (from NIST)
func TestSum256MillionA(t *testing.T) {
	input := strings.Repeat("a", 1_000_000)
	got := Sum256([]byte(input))
	gotHex := hex.EncodeToString(got[:])
	expect := "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
	if gotHex != expect {
		t.Errorf("Sum256(million a's) = %s, want %s", gotHex, expect)
	}
}

// RFC 4231 test vectors for HMAC-SHA256
// https://datatracker.ietf.org/doc/html/rfc4231
var hmacTests = []struct {
	name    string
	key     string
	message string
	expect  string
}{
	{
		name:    "RFC 4231 Test Case 1",
		key:     "0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
		message: "4869205468657265", // "Hi There"
		expect:  "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
	},
	{
		name:    "RFC 4231 Test Case 2 (key = 'Jefe')",
		key:     "4a656665", // "Jefe"
		message: "7768617420646f2079612077616e7420666f72206e6f7468696e673f", // "what do ya want for nothing?"
		expect:  "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
	},
	{
		name:    "RFC 4231 Test Case 3",
		key:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		message: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
		expect:  "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
	},
	{
		name:    "RFC 4231 Test Case 4",
		key:     "0102030405060708090a0b0c0d0e0f10111213141516171819",
		message: "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
		expect:  "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b",
	},
	{
		name:    "RFC 4231 Test Case 5 (truncated - using full)",
		key:     "0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c",
		message: "546573742057697468205472756e636174696f6e", // "Test With Truncation"
		expect:  "a3b6167473100ee06e0c796c2955552bfa6f7c0a6a8aef8b93f860aab0cd20c5",
	},
	{
		name:    "RFC 4231 Test Case 6 (key longer than block size)",
		key:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		message: "54657374205573696e67204c6172676572205468616e20426c6f636b2d53697a65204b6579202d2048617368204b6579204669727374", // "Test Using Larger Than Block-Size Key - Hash Key First"
		expect:  "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
	},
	{
		name:    "RFC 4231 Test Case 7 (key and message longer than block size)",
		key:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		message: "5468697320697320612074657374207573696e672061206c6172676572207468616e20626c6f636b2d73697a65206b657920616e642061206c6172676572207468616e20626c6f636b2d73697a6520646174612e20546865206b6579206e6565647320746f20626520686173686564206265666f7265206265696e6720757365642062792074686520484d414320616c676f726974686d2e",
		expect:  "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
	},
}

func TestHMAC(t *testing.T) {
	for _, tt := range hmacTests {
		t.Run(tt.name, func(t *testing.T) {
			key, err := hex.DecodeString(tt.key)
			if err != nil {
				t.Fatalf("failed to decode key: %v", err)
			}
			message, err := hex.DecodeString(tt.message)
			if err != nil {
				t.Fatalf("failed to decode message: %v", err)
			}

			got := HMAC(key, message)
			gotHex := hex.EncodeToString(got)
			if gotHex != tt.expect {
				t.Errorf("HMAC() = %s, want %s", gotHex, tt.expect)
			}
		})
	}
}

// Additional HMAC edge cases
func TestHMACEdgeCases(t *testing.T) {
	tests := []struct {
		name    string
		key     []byte
		message []byte
		expect  string
	}{
		{
			name:    "empty key and message",
			key:     []byte{},
			message: []byte{},
			expect:  "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad",
		},
		{
			name:    "empty message",
			key:     []byte("key"),
			message: []byte{},
			expect:  "5d5d139563c95b5967b9bd9a8c9b233a9dedb45072794cd232dc1b74832607d0",
		},
		{
			name:    "empty key",
			key:     []byte{},
			message: []byte("message"),
			expect:  "eb08c1f56d5ddee07f7bdf80468083da06b64cf4fac64fe3a90883df5feacae4",
		},
		{
			name:    "key exactly 64 bytes (block size)",
			key:     bytes.Repeat([]byte("A"), 64),
			message: []byte("test"),
			expect:  "d2e7c9d70f0cbdcc6d9fe6a51d4e16949f651c868f2f0df9310fde0ea1ebb5e3",
		},
		{
			name:    "key 65 bytes (block size + 1)",
			key:     bytes.Repeat([]byte("A"), 65),
			message: []byte("test"),
			expect:  "3f2f49de28e85b938e9495530a5cebb49b80ec5d1d86c93ff777712028e25ed1",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := HMAC(tt.key, tt.message)
			gotHex := hex.EncodeToString(got)
			if gotHex != tt.expect {
				t.Errorf("HMAC() = %s, want %s", gotHex, tt.expect)
			}
		})
	}
}

func TestConstants(t *testing.T) {
	if Size != 32 {
		t.Errorf("Size = %d, want 32", Size)
	}
	if BlockSize != 64 {
		t.Errorf("BlockSize = %d, want 64", BlockSize)
	}
}

func TestSumReturnsSlice(t *testing.T) {
	input := []byte("test")
	result := Sum(input)
	if len(result) != Size {
		t.Errorf("Sum() returned slice of length %d, want %d", len(result), Size)
	}
}

func TestSum256ReturnsArray(t *testing.T) {
	input := []byte("test")
	result := Sum256(input)
	if len(result) != Size {
		t.Errorf("Sum256() returned array of length %d, want %d", len(result), Size)
	}
}

// Benchmark tests
func BenchmarkSum256Short(b *testing.B) {
	data := []byte("hello world")
	b.SetBytes(int64(len(data)))
	for i := 0; i < b.N; i++ {
		Sum256(data)
	}
}

func BenchmarkSum256Medium(b *testing.B) {
	data := bytes.Repeat([]byte("a"), 1024)
	b.SetBytes(int64(len(data)))
	for i := 0; i < b.N; i++ {
		Sum256(data)
	}
}

func BenchmarkSum256Large(b *testing.B) {
	data := bytes.Repeat([]byte("a"), 1024*1024)
	b.SetBytes(int64(len(data)))
	for i := 0; i < b.N; i++ {
		Sum256(data)
	}
}

func BenchmarkHMAC(b *testing.B) {
	key := []byte("secret-key-for-hmac-testing")
	message := []byte("message to authenticate")
	for i := 0; i < b.N; i++ {
		HMAC(key, message)
	}
}
