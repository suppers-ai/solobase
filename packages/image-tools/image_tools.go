package imagetools

import (
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"image/gif"
	"image/jpeg"
	"image/png"
	"io"
	"math"
	"strings"

	"golang.org/x/image/bmp"
	xdraw "golang.org/x/image/draw"
	"golang.org/x/image/tiff"
)

// ImageFormat represents supported image formats
type ImageFormat string

const (
	FormatJPEG ImageFormat = "jpeg"
	FormatPNG  ImageFormat = "png"
	FormatGIF  ImageFormat = "gif"
	FormatBMP  ImageFormat = "bmp"
	FormatTIFF ImageFormat = "tiff"
	FormatWEBP ImageFormat = "webp"
)

// ResizeOptions contains options for resizing images
type ResizeOptions struct {
	Width      int         // Target width (0 to maintain aspect ratio)
	Height     int         // Target height (0 to maintain aspect ratio)
	Quality    int         // JPEG quality (1-100, default 95)
	Filter     Filter      // Resampling filter
	Format     ImageFormat // Output format (auto-detect if empty)
	KeepAspect bool        // Keep aspect ratio (default true)
}

// Filter represents resampling filters
type Filter int

const (
	FilterNearestNeighbor Filter = iota
	FilterBilinear
	FilterBicubic
	FilterCatmullRom
	FilterApproxBiLinear
)

// ImageProcessor provides image manipulation methods
type ImageProcessor struct {
	defaultQuality int
	maxWidth       int
	maxHeight      int
}

// NewImageProcessor creates a new image processor with default settings
func NewImageProcessor() *ImageProcessor {
	return &ImageProcessor{
		defaultQuality: 95,
		maxWidth:       10000,
		maxHeight:      10000,
	}
}

// Resize resizes an image from the reader and writes to the writer
func (p *ImageProcessor) Resize(reader io.Reader, writer io.Writer, options ResizeOptions) error {
	// Set defaults
	if options.Quality == 0 {
		options.Quality = p.defaultQuality
	}
	if options.Quality < 1 || options.Quality > 100 {
		options.Quality = 95
	}

	// Decode image
	img, format, err := image.Decode(reader)
	if err != nil {
		return fmt.Errorf("failed to decode image: %v", err)
	}

	// Auto-detect format if not specified
	if options.Format == "" {
		options.Format = ImageFormat(format)
	}

	// Get original dimensions
	bounds := img.Bounds()
	origWidth := bounds.Dx()
	origHeight := bounds.Dy()

	// Calculate target dimensions
	targetWidth, targetHeight := p.calculateDimensions(
		origWidth, origHeight,
		options.Width, options.Height,
		options.KeepAspect,
	)

	// Create resized image
	resized := p.resizeImage(img, targetWidth, targetHeight, options.Filter)

	// Encode the image
	return p.encode(writer, resized, options.Format, options.Quality)
}

// resizeImage performs the actual resizing using the standard library
func (p *ImageProcessor) resizeImage(src image.Image, width, height int, filter Filter) image.Image {
	dst := image.NewRGBA(image.Rect(0, 0, width, height))

	// Use the appropriate scaler from x/image/draw
	switch filter {
	case FilterNearestNeighbor:
		xdraw.NearestNeighbor.Scale(dst, dst.Bounds(), src, src.Bounds(), xdraw.Over, nil)
	case FilterBilinear:
		xdraw.BiLinear.Scale(dst, dst.Bounds(), src, src.Bounds(), xdraw.Over, nil)
	case FilterApproxBiLinear:
		xdraw.ApproxBiLinear.Scale(dst, dst.Bounds(), src, src.Bounds(), xdraw.Over, nil)
	case FilterBicubic, FilterCatmullRom:
		xdraw.CatmullRom.Scale(dst, dst.Bounds(), src, src.Bounds(), xdraw.Over, nil)
	default:
		xdraw.CatmullRom.Scale(dst, dst.Bounds(), src, src.Bounds(), xdraw.Over, nil)
	}

	return dst
}

// calculateDimensions calculates the target dimensions for resizing
func (p *ImageProcessor) calculateDimensions(origWidth, origHeight, targetWidth, targetHeight int, keepAspect bool) (int, int) {
	// If both dimensions are specified and we don't need to keep aspect ratio
	if targetWidth > 0 && targetHeight > 0 && !keepAspect {
		return targetWidth, targetHeight
	}

	// Calculate aspect ratio
	aspectRatio := float64(origWidth) / float64(origHeight)

	// If both dimensions are specified and we need to keep aspect ratio
	if targetWidth > 0 && targetHeight > 0 && keepAspect {
		// Fit within the bounds
		widthRatio := float64(targetWidth) / float64(origWidth)
		heightRatio := float64(targetHeight) / float64(origHeight)

		if widthRatio < heightRatio {
			return targetWidth, int(float64(targetWidth) / aspectRatio)
		}
		return int(float64(targetHeight) * aspectRatio), targetHeight
	}

	// If only width is specified
	if targetWidth > 0 {
		return targetWidth, int(float64(targetWidth) / aspectRatio)
	}

	// If only height is specified
	if targetHeight > 0 {
		return int(float64(targetHeight) * aspectRatio), targetHeight
	}

	// No resize needed
	return origWidth, origHeight
}

// ResizeToBytes resizes an image and returns the result as bytes
func (p *ImageProcessor) ResizeToBytes(input []byte, options ResizeOptions) ([]byte, error) {
	reader := bytes.NewReader(input)
	var output bytes.Buffer

	if err := p.Resize(reader, &output, options); err != nil {
		return nil, err
	}

	return output.Bytes(), nil
}

// Thumbnail creates a thumbnail with specified maximum dimensions
func (p *ImageProcessor) Thumbnail(reader io.Reader, writer io.Writer, maxWidth, maxHeight int) error {
	return p.Resize(reader, writer, ResizeOptions{
		Width:      maxWidth,
		Height:     maxHeight,
		KeepAspect: true,
		Filter:     FilterBicubic,
	})
}

// Crop crops an image to the specified dimensions
func (p *ImageProcessor) Crop(reader io.Reader, writer io.Writer, x, y, width, height int, format ImageFormat) error {
	// Decode image
	img, origFormat, err := image.Decode(reader)
	if err != nil {
		return fmt.Errorf("failed to decode image: %v", err)
	}

	// Auto-detect format if not specified
	if format == "" {
		format = ImageFormat(origFormat)
	}

	// Create cropped image
	cropRect := image.Rect(x, y, x+width, y+height)
	cropped := image.NewRGBA(image.Rect(0, 0, width, height))
	draw.Draw(cropped, cropped.Bounds(), img, cropRect.Min, draw.Src)

	// Encode the image
	return p.encode(writer, cropped, format, p.defaultQuality)
}

// Rotate rotates an image by the specified angle (in degrees)
func (p *ImageProcessor) Rotate(reader io.Reader, writer io.Writer, angle float64, format ImageFormat) error {
	// Decode image
	img, origFormat, err := image.Decode(reader)
	if err != nil {
		return fmt.Errorf("failed to decode image: %v", err)
	}

	// Auto-detect format if not specified
	if format == "" {
		format = ImageFormat(origFormat)
	}

	// Normalize angle to 0-360
	angle = math.Mod(angle, 360)
	if angle < 0 {
		angle += 360
	}

	var rotated image.Image

	// Handle special cases for 90-degree rotations (faster)
	switch angle {
	case 0:
		rotated = img
	case 90:
		rotated = p.rotate90(img)
	case 180:
		rotated = p.rotate180(img)
	case 270:
		rotated = p.rotate270(img)
	default:
		rotated = p.rotateArbitrary(img, angle)
	}

	// Encode the image
	return p.encode(writer, rotated, format, p.defaultQuality)
}

// rotate90 rotates an image 90 degrees clockwise
func (p *ImageProcessor) rotate90(src image.Image) image.Image {
	bounds := src.Bounds()
	width := bounds.Dy()
	height := bounds.Dx()

	dst := image.NewRGBA(image.Rect(0, 0, width, height))

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			dstX := bounds.Max.Y - 1 - y
			dstY := x - bounds.Min.X
			dst.Set(dstX, dstY, src.At(x, y))
		}
	}

	return dst
}

// rotate180 rotates an image 180 degrees
func (p *ImageProcessor) rotate180(src image.Image) image.Image {
	bounds := src.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()

	dst := image.NewRGBA(image.Rect(0, 0, width, height))

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			dstX := bounds.Max.X - 1 - x
			dstY := bounds.Max.Y - 1 - y
			dst.Set(dstX, dstY, src.At(x, y))
		}
	}

	return dst
}

// rotate270 rotates an image 270 degrees clockwise (90 degrees counter-clockwise)
func (p *ImageProcessor) rotate270(src image.Image) image.Image {
	bounds := src.Bounds()
	width := bounds.Dy()
	height := bounds.Dx()

	dst := image.NewRGBA(image.Rect(0, 0, width, height))

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			dstX := y - bounds.Min.Y
			dstY := bounds.Max.X - 1 - x
			dst.Set(dstX, dstY, src.At(x, y))
		}
	}

	return dst
}

// rotateArbitrary rotates an image by an arbitrary angle
func (p *ImageProcessor) rotateArbitrary(src image.Image, angleDegrees float64) image.Image {
	angle := angleDegrees * math.Pi / 180

	bounds := src.Bounds()
	srcWidth := bounds.Dx()
	srcHeight := bounds.Dy()

	// Calculate new dimensions
	sin := math.Abs(math.Sin(angle))
	cos := math.Abs(math.Cos(angle))
	newWidth := int(float64(srcWidth)*cos + float64(srcHeight)*sin)
	newHeight := int(float64(srcWidth)*sin + float64(srcHeight)*cos)

	dst := image.NewRGBA(image.Rect(0, 0, newWidth, newHeight))

	// Center points
	srcCenterX := float64(srcWidth) / 2
	srcCenterY := float64(srcHeight) / 2
	dstCenterX := float64(newWidth) / 2
	dstCenterY := float64(newHeight) / 2

	// Rotation matrix (inverse for sampling)
	cos = math.Cos(-angle)
	sin = math.Sin(-angle)

	for y := 0; y < newHeight; y++ {
		for x := 0; x < newWidth; x++ {
			// Translate to origin
			xf := float64(x) - dstCenterX
			yf := float64(y) - dstCenterY

			// Rotate
			srcX := xf*cos - yf*sin + srcCenterX
			srcY := xf*sin + yf*cos + srcCenterY

			// Sample from source image with bilinear interpolation
			if srcX >= 0 && srcX < float64(srcWidth) && srcY >= 0 && srcY < float64(srcHeight) {
				dst.Set(x, y, p.bilinearSample(src, srcX, srcY))
			} else {
				dst.Set(x, y, color.RGBA{0, 0, 0, 0}) // Transparent
			}
		}
	}

	return dst
}

// bilinearSample performs bilinear interpolation for smoother rotation
func (p *ImageProcessor) bilinearSample(img image.Image, x, y float64) color.Color {
	bounds := img.Bounds()

	x0 := int(math.Floor(x))
	y0 := int(math.Floor(y))
	x1 := x0 + 1
	y1 := y0 + 1

	// Clamp to image bounds
	if x0 < bounds.Min.X {
		x0 = bounds.Min.X
	}
	if y0 < bounds.Min.Y {
		y0 = bounds.Min.Y
	}
	if x1 >= bounds.Max.X {
		x1 = bounds.Max.X - 1
	}
	if y1 >= bounds.Max.Y {
		y1 = bounds.Max.Y - 1
	}

	// Get the fractional parts
	fx := x - math.Floor(x)
	fy := y - math.Floor(y)

	// Get the four pixels
	c00 := img.At(x0, y0)
	c01 := img.At(x0, y1)
	c10 := img.At(x1, y0)
	c11 := img.At(x1, y1)

	// Convert to RGBA
	r00, g00, b00, a00 := c00.RGBA()
	r01, g01, b01, a01 := c01.RGBA()
	r10, g10, b10, a10 := c10.RGBA()
	r11, g11, b11, a11 := c11.RGBA()

	// Bilinear interpolation
	r := uint32((1-fx)*(1-fy)*float64(r00) + fx*(1-fy)*float64(r10) + (1-fx)*fy*float64(r01) + fx*fy*float64(r11))
	g := uint32((1-fx)*(1-fy)*float64(g00) + fx*(1-fy)*float64(g10) + (1-fx)*fy*float64(g01) + fx*fy*float64(g11))
	b := uint32((1-fx)*(1-fy)*float64(b00) + fx*(1-fy)*float64(b10) + (1-fx)*fy*float64(b01) + fx*fy*float64(b11))
	a := uint32((1-fx)*(1-fy)*float64(a00) + fx*(1-fy)*float64(a10) + (1-fx)*fy*float64(a01) + fx*fy*float64(a11))

	return color.RGBA64{uint16(r), uint16(g), uint16(b), uint16(a)}
}

// GetImageInfo returns information about an image
func (p *ImageProcessor) GetImageInfo(reader io.Reader) (*ImageInfo, error) {
	img, format, err := image.Decode(reader)
	if err != nil {
		return nil, fmt.Errorf("failed to decode image: %v", err)
	}

	bounds := img.Bounds()
	return &ImageInfo{
		Width:  bounds.Dx(),
		Height: bounds.Dy(),
		Format: ImageFormat(format),
	}, nil
}

// ImageInfo contains information about an image
type ImageInfo struct {
	Width  int
	Height int
	Format ImageFormat
}

// encode encodes an image to the writer in the specified format
func (p *ImageProcessor) encode(writer io.Writer, img image.Image, format ImageFormat, quality int) error {
	switch strings.ToLower(string(format)) {
	case "jpeg", "jpg":
		return jpeg.Encode(writer, img, &jpeg.Options{Quality: quality})
	case "png":
		return png.Encode(writer, img)
	case "gif":
		return gif.Encode(writer, img, nil)
	case "bmp":
		return bmp.Encode(writer, img)
	case "tiff":
		return tiff.Encode(writer, img, nil)
	case "webp":
		// Note: webp encoding requires additional setup
		// For now, fallback to PNG
		return png.Encode(writer, img)
	default:
		return fmt.Errorf("unsupported format: %s", format)
	}
}

// Helper functions for common operations

// ResizeToWidth resizes an image to a specific width maintaining aspect ratio
func ResizeToWidth(input []byte, width int, quality int) ([]byte, error) {
	processor := NewImageProcessor()
	return processor.ResizeToBytes(input, ResizeOptions{
		Width:   width,
		Quality: quality,
		Filter:  FilterBicubic,
	})
}

// ResizeToHeight resizes an image to a specific height maintaining aspect ratio
func ResizeToHeight(input []byte, height int, quality int) ([]byte, error) {
	processor := NewImageProcessor()
	return processor.ResizeToBytes(input, ResizeOptions{
		Height:  height,
		Quality: quality,
		Filter:  FilterBicubic,
	})
}

// CreateThumbnail creates a thumbnail from image bytes
func CreateThumbnail(input []byte, maxWidth, maxHeight int) ([]byte, error) {
	processor := NewImageProcessor()
	return processor.ResizeToBytes(input, ResizeOptions{
		Width:      maxWidth,
		Height:     maxHeight,
		KeepAspect: true,
		Quality:    85,
		Filter:     FilterBicubic,
	})
}

// DetectFormat detects the format of an image from its bytes
func DetectFormat(data []byte) (ImageFormat, error) {
	reader := bytes.NewReader(data)
	_, format, err := image.DecodeConfig(reader)
	if err != nil {
		return "", fmt.Errorf("failed to detect image format: %v", err)
	}
	return ImageFormat(format), nil
}
