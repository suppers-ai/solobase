package imagetools

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
	"testing"
)

// createTestImage creates a test image for testing
func createTestImage(width, height int) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))

	// Draw a simple pattern
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			c := color.RGBA{
				R: uint8((x * 255) / width),
				G: uint8((y * 255) / height),
				B: 128,
				A: 255,
			}
			img.Set(x, y, c)
		}
	}

	// Encode to PNG
	var buf bytes.Buffer
	png.Encode(&buf, img)
	return buf.Bytes()
}

func TestResize(t *testing.T) {
	processor := NewImageProcessor()

	// Create a test image
	testImg := createTestImage(800, 600)

	tests := []struct {
		name         string
		options      ResizeOptions
		expectWidth  int
		expectHeight int
	}{
		{
			name: "Resize to specific width",
			options: ResizeOptions{
				Width:  400,
				Filter: FilterBicubic,
			},
			expectWidth:  400,
			expectHeight: 300, // Should maintain aspect ratio
		},
		{
			name: "Resize to specific height",
			options: ResizeOptions{
				Height: 300,
				Filter: FilterBicubic,
			},
			expectWidth:  400, // Should maintain aspect ratio
			expectHeight: 300,
		},
		{
			name: "Resize to fit within bounds",
			options: ResizeOptions{
				Width:      300,
				Height:     300,
				KeepAspect: true,
				Filter:     FilterBicubic,
			},
			expectWidth:  300,
			expectHeight: 225, // Maintains 4:3 aspect ratio
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			reader := bytes.NewReader(testImg)
			var output bytes.Buffer

			err := processor.Resize(reader, &output, tt.options)
			if err != nil {
				t.Fatalf("Failed to resize image: %v", err)
			}

			// Decode the result to check dimensions
			result, _, err := image.Decode(&output)
			if err != nil {
				t.Fatalf("Failed to decode resized image: %v", err)
			}

			bounds := result.Bounds()
			width := bounds.Dx()
			height := bounds.Dy()

			// Allow for small differences due to aspect ratio calculations
			if abs(width-tt.expectWidth) > 1 || abs(height-tt.expectHeight) > 1 {
				t.Errorf("Expected dimensions %dx%d, got %dx%d",
					tt.expectWidth, tt.expectHeight, width, height)
			}
		})
	}
}

func TestThumbnail(t *testing.T) {
	processor := NewImageProcessor()

	// Create a test image
	testImg := createTestImage(1920, 1080)

	reader := bytes.NewReader(testImg)
	var output bytes.Buffer

	err := processor.Thumbnail(reader, &output, 200, 200)
	if err != nil {
		t.Fatalf("Failed to create thumbnail: %v", err)
	}

	// Decode the result to check dimensions
	result, _, err := image.Decode(&output)
	if err != nil {
		t.Fatalf("Failed to decode thumbnail: %v", err)
	}

	bounds := result.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()

	// Should fit within 200x200 while maintaining aspect ratio
	if width > 200 || height > 200 {
		t.Errorf("Thumbnail exceeds maximum dimensions: %dx%d", width, height)
	}

	// Check aspect ratio is maintained (16:9)
	aspectRatio := float64(width) / float64(height)
	expectedRatio := 16.0 / 9.0
	if abs(int(aspectRatio*100-expectedRatio*100)) > 2 {
		t.Errorf("Aspect ratio not maintained: expected ~%f, got %f", expectedRatio, aspectRatio)
	}
}

func TestGetImageInfo(t *testing.T) {
	processor := NewImageProcessor()

	// Create a test image
	testImg := createTestImage(640, 480)

	reader := bytes.NewReader(testImg)
	info, err := processor.GetImageInfo(reader)
	if err != nil {
		t.Fatalf("Failed to get image info: %v", err)
	}

	if info.Width != 640 || info.Height != 480 {
		t.Errorf("Expected dimensions 640x480, got %dx%d", info.Width, info.Height)
	}

	if info.Format != FormatPNG {
		t.Errorf("Expected format PNG, got %s", info.Format)
	}
}

func TestDetectFormat(t *testing.T) {
	// Create a PNG image
	testImg := createTestImage(100, 100)

	format, err := DetectFormat(testImg)
	if err != nil {
		t.Fatalf("Failed to detect format: %v", err)
	}

	if format != FormatPNG {
		t.Errorf("Expected format PNG, got %s", format)
	}
}

func TestCreateThumbnail(t *testing.T) {
	// Create a test image
	testImg := createTestImage(800, 600)

	thumbnail, err := CreateThumbnail(testImg, 150, 150)
	if err != nil {
		t.Fatalf("Failed to create thumbnail: %v", err)
	}

	// Decode the thumbnail to check dimensions
	reader := bytes.NewReader(thumbnail)
	result, _, err := image.Decode(reader)
	if err != nil {
		t.Fatalf("Failed to decode thumbnail: %v", err)
	}

	bounds := result.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()

	// Should fit within 150x150
	if width > 150 || height > 150 {
		t.Errorf("Thumbnail exceeds maximum dimensions: %dx%d", width, height)
	}
}

// Helper function for absolute value
func abs(n int) int {
	if n < 0 {
		return -n
	}
	return n
}
