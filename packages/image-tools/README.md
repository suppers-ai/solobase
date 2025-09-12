# Image Tools Package

A Go package for image manipulation including resizing, cropping, rotating, and format conversion.

## Features

- **Resize images** with multiple resampling filters
- **Create thumbnails** maintaining aspect ratio
- **Crop images** to specific dimensions
- **Rotate images** by any angle
- **Format conversion** between JPEG, PNG, GIF, BMP, TIFF, and WebP
- **Get image information** (dimensions, format)
- **Batch processing** support

## Installation

```bash
go get github.com/suppers-ai/image-tools
```

## Usage

### Basic Resize

```go
package main

import (
    "os"
    imagetools "github.com/suppers-ai/image-tools"
)

func main() {
    processor := imagetools.NewImageProcessor()
    
    // Open input file
    input, _ := os.Open("input.jpg")
    defer input.Close()
    
    // Create output file
    output, _ := os.Create("output.jpg")
    defer output.Close()
    
    // Resize to 800px width, maintaining aspect ratio
    err := processor.Resize(input, output, imagetools.ResizeOptions{
        Width:   800,
        Quality: 90,
        Filter:  imagetools.FilterLanczos,
    })
}
```

### Create Thumbnail

```go
// Quick thumbnail creation
inputBytes, _ := os.ReadFile("large-image.jpg")
thumbnail, err := imagetools.CreateThumbnail(inputBytes, 200, 200)
if err == nil {
    os.WriteFile("thumbnail.jpg", thumbnail, 0644)
}
```

### Resize with Different Options

```go
processor := imagetools.NewImageProcessor()

// Resize to exact dimensions (may distort)
processor.Resize(input, output, imagetools.ResizeOptions{
    Width:      800,
    Height:     600,
    KeepAspect: false,
    Quality:    95,
})

// Fit within bounds maintaining aspect ratio
processor.Resize(input, output, imagetools.ResizeOptions{
    Width:      800,
    Height:     600,
    KeepAspect: true,
    Filter:     imagetools.FilterCubic,
})

// Convert format while resizing
processor.Resize(input, output, imagetools.ResizeOptions{
    Width:   1024,
    Format:  imagetools.FormatPNG,
})
```

### Crop Image

```go
processor := imagetools.NewImageProcessor()

// Crop a 200x200 square starting at position (100, 50)
err := processor.Crop(input, output, 100, 50, 200, 200, imagetools.FormatJPEG)
```

### Rotate Image

```go
processor := imagetools.NewImageProcessor()

// Rotate 90 degrees clockwise
err := processor.Rotate(input, output, 90, imagetools.FormatPNG)
```

### Get Image Information

```go
processor := imagetools.NewImageProcessor()

info, err := processor.GetImageInfo(input)
if err == nil {
    fmt.Printf("Image: %dx%d, Format: %s\n", 
        info.Width, info.Height, info.Format)
}
```

## Resampling Filters

The package supports multiple resampling filters for different quality/performance trade-offs:

- `FilterNearestNeighbor` - Fastest, lowest quality
- `FilterBox` - Fast, low quality
- `FilterLinear` - Good balance
- `FilterCubic` - High quality
- `FilterLanczos` - Highest quality (default for thumbnails)

## Supported Formats

- JPEG/JPG
- PNG
- GIF
- BMP
- TIFF
- WebP (read only, write converts to PNG)

## Performance Tips

1. Use appropriate filters - `FilterBox` for speed, `FilterLanczos` for quality
2. Adjust JPEG quality based on needs (85-95 for high quality, 70-85 for web)
3. Consider using goroutines for batch processing
4. Reuse `ImageProcessor` instances

## Example: Batch Processing

```go
func processBatch(files []string) {
    processor := imagetools.NewImageProcessor()
    
    var wg sync.WaitGroup
    for _, file := range files {
        wg.Add(1)
        go func(filename string) {
            defer wg.Done()
            
            input, _ := os.ReadFile(filename)
            resized, err := processor.ResizeToBytes(input, imagetools.ResizeOptions{
                Width:   1200,
                Quality: 85,
            })
            
            if err == nil {
                outputName := "resized_" + filename
                os.WriteFile(outputName, resized, 0644)
            }
        }(file)
    }
    wg.Wait()
}
```

## License

MIT