# Design Document

## Overview

The Paint application is a web-based drawing tool built using the Fresh framework with Preact, following the same architectural patterns as the existing Recorder application. The application provides an intuitive interface for digital artwork creation with HTML5 Canvas API for drawing operations, customizable tools, and image insertion capabilities.

The application will be structured as a Fresh application within the `applications/paint` directory, utilizing the existing UI library components and authentication system from the Suppers platform.

## Architecture

### Application Structure
```
applications/paint/
├── main.ts                    # Application entry point
├── dev.ts                     # Development server
├── deno.json                  # Package configuration
├── routes/
│   ├── index.tsx             # Main paint interface
│   └── api/
│       └── paintings.ts      # API endpoints for saving/loading
├── islands/
│   ├── PaintCanvasIsland.tsx # Main drawing canvas component
│   ├── ToolbarIsland.tsx     # Drawing tools and controls
│   └── SimpleNavbar.tsx      # Navigation (reused from recorder)
├── components/
│   └── Layout.tsx            # Layout wrapper (reused from recorder)
├── lib/
│   ├── paint-utils.ts        # Canvas drawing utilities
│   ├── auth.ts               # Authentication utilities (reused)
│   └── api-utils.ts          # API communication utilities
├── types/
│   └── paint.ts              # TypeScript interfaces
└── static/
    └── favicon.ico           # Application favicon
```

### Technology Stack
- **Canvas API**: HTML5 Canvas for drawing operations
- **Fresh Framework**: Server-side rendering and routing
- **Preact**: Client-side interactivity for islands
- **Supabase**: Backend storage for saved paintings
- **DaisyUI + Tailwind**: UI styling consistent with platform
- **TypeScript**: Type safety throughout the application

## Components and Interfaces

### Core Components

#### 1. PaintCanvasIsland
The main interactive component handling all drawing operations.

**Responsibilities:**
- Canvas initialization and management
- Mouse/touch event handling for drawing
- Stroke rendering with current tool settings
- Image insertion and positioning
- Undo/redo functionality
- Canvas export for saving

**State Management:**
```typescript
interface CanvasState {
  isDrawing: boolean;
  currentTool: 'pencil' | 'eraser' | 'image';
  pencilColor: string;
  pencilWidth: number;
  canvasHistory: ImageData[];
  historyStep: number;
  insertedImages: InsertedImage[];
}
```

#### 2. ToolbarIsland
Tool selection and configuration interface.

**Features:**
- Color picker for pencil color
- Width slider for pencil thickness
- Tool selection (pencil, eraser, image insertion)
- Action buttons (clear, undo, redo, save)
- Responsive layout for mobile devices

#### 3. Layout Component
Reused from the recorder application with minimal modifications for paint-specific branding.

### Data Models

#### Drawing State
```typescript
interface DrawingState {
  strokes: Stroke[];
  images: InsertedImage[];
  canvasSize: { width: number; height: number };
  backgroundColor: string;
}

interface Stroke {
  id: string;
  points: Point[];
  color: string;
  width: number;
  timestamp: number;
}

interface Point {
  x: number;
  y: number;
  pressure?: number; // For future stylus support
}

interface InsertedImage {
  id: string;
  src: string;
  x: number;
  y: number;
  width: number;
  height: number;
  timestamp: number;
}
```

#### Saved Painting
```typescript
interface SavedPainting {
  id: string;
  name: string;
  userId: string;
  drawingData: DrawingState;
  thumbnail: string; // Base64 encoded thumbnail
  createdAt: Date;
  updatedAt: Date;
  size: number; // File size in bytes
  isPublic: boolean;
}
```

### Canvas Drawing System

#### Drawing Engine
The drawing system will use HTML5 Canvas with the following approach:

1. **Event Handling**: Mouse and touch events for cross-device compatibility
2. **Smooth Lines**: Quadratic curves between points for smooth strokes
3. **Performance**: Efficient rendering with requestAnimationFrame
4. **Memory Management**: Canvas history management for undo/redo

#### Drawing Operations
```typescript
interface DrawingOperations {
  startStroke(point: Point): void;
  addPointToStroke(point: Point): void;
  endStroke(): void;
  drawStroke(stroke: Stroke): void;
  clearCanvas(): void;
  insertImage(file: File, position: Point): Promise<void>;
  exportCanvas(): Blob;
}
```

## Error Handling

### Canvas Errors
- **Browser Compatibility**: Fallback message for unsupported browsers
- **Memory Limits**: Warning when canvas history becomes too large
- **Image Loading**: Error handling for invalid or corrupted image files
- **File Size Limits**: Validation for uploaded images

### API Errors
- **Authentication**: Token expiration and refresh handling
- **Network**: Retry logic for failed save operations
- **Storage**: Quota exceeded warnings
- **Validation**: Input validation for painting data

### User Experience
- **Loading States**: Visual feedback during save/load operations
- **Error Messages**: User-friendly error notifications
- **Recovery**: Auto-save functionality to prevent data loss
- **Offline Support**: Local storage fallback when offline

## Testing Strategy

### Unit Tests
- Canvas utility functions
- Drawing calculations and transformations
- Image processing operations
- Data serialization/deserialization

### Integration Tests
- Canvas drawing operations
- Tool interactions
- Save/load functionality
- Authentication integration

### E2E Tests
- Complete drawing workflows
- Cross-device compatibility
- Performance under load
- Error recovery scenarios

### Visual Tests
- Canvas rendering accuracy
- UI component consistency
- Responsive design validation
- Color accuracy across devices

## Performance Considerations

### Canvas Optimization
- **Efficient Rendering**: Only redraw changed areas when possible
- **Memory Management**: Limit undo history to prevent memory leaks
- **Image Optimization**: Compress inserted images for storage
- **Debounced Operations**: Throttle rapid drawing events

### Storage Optimization
- **Compression**: Compress drawing data before storage
- **Thumbnails**: Generate small previews for gallery views
- **Lazy Loading**: Load paintings on demand
- **Caching**: Cache frequently accessed paintings

### Mobile Performance
- **Touch Optimization**: Smooth touch tracking on mobile devices
- **Battery Efficiency**: Minimize unnecessary redraws
- **Memory Constraints**: Adapt canvas size for mobile devices
- **Network Efficiency**: Optimize API calls for mobile networks

## Security Considerations

### Input Validation
- **Image Files**: Validate file types and sizes for uploaded images
- **Canvas Data**: Sanitize drawing data before storage
- **User Input**: Validate all user inputs and tool settings

### Authentication
- **Session Management**: Secure token handling and refresh
- **Authorization**: Ensure users can only access their own paintings
- **Rate Limiting**: Prevent abuse of save/load operations

### Data Protection
- **Encryption**: Encrypt sensitive painting data in storage
- **Privacy**: Respect user privacy settings for public/private paintings
- **Backup**: Regular backups of user artwork

## Accessibility

### Keyboard Navigation
- **Tool Selection**: Keyboard shortcuts for common tools
- **Canvas Navigation**: Arrow keys for precise positioning
- **Menu Access**: Tab navigation through all interface elements

### Screen Reader Support
- **ARIA Labels**: Descriptive labels for all interactive elements
- **Status Updates**: Announce drawing actions and tool changes
- **Alternative Text**: Descriptions for visual elements

### Visual Accessibility
- **High Contrast**: Support for high contrast themes
- **Color Blind Support**: Alternative indicators beyond color
- **Zoom Support**: Canvas scaling for users with visual impairments

## Deployment and Scaling

### Fresh Application Deployment
- **Docker Container**: Containerized deployment following platform patterns
- **Environment Configuration**: Separate configs for dev/staging/production
- **Static Assets**: Efficient serving of canvas-related assets

### Database Schema
```sql
-- Paintings table
CREATE TABLE paintings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES auth.users(id),
  name TEXT NOT NULL,
  drawing_data JSONB NOT NULL,
  thumbnail TEXT, -- Base64 encoded thumbnail
  file_size INTEGER DEFAULT 0,
  is_public BOOLEAN DEFAULT false,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_paintings_user_id ON paintings(user_id);
CREATE INDEX idx_paintings_created_at ON paintings(created_at DESC);
CREATE INDEX idx_paintings_public ON paintings(is_public) WHERE is_public = true;
```

### API Endpoints
- `GET /api/paintings` - List user's paintings
- `POST /api/paintings` - Save new painting
- `PUT /api/paintings/:id` - Update existing painting
- `DELETE /api/paintings/:id` - Delete painting
- `GET /api/paintings/:id/download` - Download painting data
- `POST /api/paintings/upload-image` - Upload image for insertion

This design provides a comprehensive foundation for building a feature-rich paint application that integrates seamlessly with the existing Suppers platform while providing an intuitive and performant drawing experience across all devices.