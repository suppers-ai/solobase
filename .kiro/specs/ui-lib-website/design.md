# Design Document

## Overview

The UI Library Showcase Website will be a single-page Fresh 2.0 application that displays all available UI library components with interactive examples. The site will be built using the existing project structure and dependencies, leveraging Fresh 2.0, Preact, and Tailwind CSS to create a clean, responsive showcase of the Button, Input, Card, and Layout components.

## Architecture

### Technology Stack
- **Framework**: Fresh 2.0 (already configured in the project)
- **UI Library**: Preact with signals for interactivity
- **Styling**: Tailwind CSS (already configured)
- **Components**: Direct imports from `@json-app-compiler/ui-library`
- **Build System**: Deno with existing workspace configuration

### Project Structure
```
ui-lib-website/
├── deno.json                 # Fresh 2.0 configuration
├── dev.ts                    # Development server
├── main.ts                   # Production server
├── fresh.config.ts           # Fresh configuration
├── routes/
│   └── index.tsx            # Main showcase page
├── islands/
│   ├── ComponentShowcase.tsx # Interactive component demos
│   └── CodeExample.tsx      # Code display with copy functionality
├── components/
│   ├── ComponentSection.tsx  # Section wrapper for each component
│   └── PropsTable.tsx       # Props documentation display
└── static/
    └── styles.css           # Additional custom styles
```

## Components and Interfaces

### Main Page Structure

#### ComponentShowcase Island
```typescript
interface ComponentShowcaseProps {
  componentName: string;
  description: string;
  examples: ComponentExample[];
}

interface ComponentExample {
  title: string;
  description: string;
  component: ComponentChildren;
  code: string;
  props?: Record<string, any>;
}
```

The main interactive island that renders component examples with real-time prop manipulation.

#### CodeExample Island
```typescript
interface CodeExampleProps {
  code: string;
  language: 'tsx' | 'json';
  title?: string;
}
```

Displays syntax-highlighted code with copy-to-clipboard functionality.

#### ComponentSection Component
```typescript
interface ComponentSectionProps {
  title: string;
  description: string;
  children: ComponentChildren;
}
```

Wrapper component that provides consistent styling and layout for each component section.

#### PropsTable Component
```typescript
interface PropsTableProps {
  props: PropDefinition[];
}

interface PropDefinition {
  name: string;
  type: string;
  description: string;
  defaultValue?: string;
  required?: boolean;
}
```

Displays component props in a clean, readable table format.

## Data Models

### Component Configuration
```typescript
interface ComponentConfig {
  name: string;
  description: string;
  category: 'form' | 'layout' | 'display';
  examples: ComponentExample[];
  props: PropDefinition[];
}
```

### Example Configurations

#### Button Examples
```typescript
const buttonExamples: ComponentExample[] = [
  {
    title: "Button Variants",
    description: "Different visual styles for various use cases",
    component: (
      <div className="flex gap-2 flex-wrap">
        <Button variant="primary">Primary</Button>
        <Button variant="secondary">Secondary</Button>
        <Button variant="outline">Outline</Button>
        <Button variant="ghost">Ghost</Button>
        <Button variant="danger">Danger</Button>
      </div>
    ),
    code: `<Button variant="primary">Primary</Button>
<Button variant="secondary">Secondary</Button>
<Button variant="outline">Outline</Button>`
  },
  {
    title: "Button Sizes",
    description: "Small, medium, and large button sizes",
    component: (
      <div className="flex gap-2 items-center">
        <Button size="sm">Small</Button>
        <Button size="md">Medium</Button>
        <Button size="lg">Large</Button>
      </div>
    ),
    code: `<Button size="sm">Small</Button>
<Button size="md">Medium</Button>
<Button size="lg">Large</Button>`
  }
];
```

## Error Handling

### Component Error Boundaries
- Wrap each component showcase in error boundaries to prevent one broken example from crashing the entire page
- Display helpful error messages when components fail to render
- Provide fallback UI that maintains page functionality

### Code Copy Error Handling
- Handle clipboard API failures gracefully
- Provide visual feedback for successful/failed copy operations
- Fall back to text selection if clipboard API is unavailable

## Testing Strategy

### Component Testing
```typescript
// Test component rendering
test("ComponentShowcase renders all button variants", () => {
  const { container } = render(
    <ComponentShowcase 
      componentName="Button" 
      examples={buttonExamples} 
    />
  );
  
  expect(container.querySelectorAll('button')).toHaveLength(5);
});

// Test interactive features
test("CodeExample copies code to clipboard", async () => {
  const { getByText } = render(
    <CodeExample code="<Button>Test</Button>" />
  );
  
  const copyButton = getByText('Copy');
  fireEvent.click(copyButton);
  
  // Assert clipboard functionality
});
```

### Integration Testing
- Test that all UI library components render correctly
- Verify responsive behavior across different screen sizes
- Test accessibility features and keyboard navigation

### Visual Testing
- Ensure component examples match expected visual appearance
- Test responsive layout on mobile and desktop
- Verify proper spacing and alignment

## Implementation Approach

### Phase 1: Basic Structure
1. Set up Fresh 2.0 application structure
2. Create main page layout with navigation
3. Import and display basic component examples
4. Implement responsive grid layout

### Phase 2: Interactive Features
1. Add ComponentShowcase islands for each component type
2. Implement code display with syntax highlighting
3. Add copy-to-clipboard functionality
4. Create props documentation tables

### Phase 3: Polish and Optimization
1. Add smooth scrolling and navigation
2. Implement proper error boundaries
3. Optimize for performance and accessibility
4. Add responsive design refinements

## Responsive Design

### Mobile Layout (< 768px)
- Single column layout
- Stacked component examples
- Collapsible code sections
- Touch-friendly interactive elements

### Tablet Layout (768px - 1024px)
- Two-column layout where appropriate
- Larger component preview areas
- Side-by-side code and preview

### Desktop Layout (> 1024px)
- Multi-column layout for component variants
- Larger code examples with better readability
- More efficient use of horizontal space

## Accessibility Considerations

### Keyboard Navigation
- All interactive elements accessible via keyboard
- Proper tab order through component examples
- Skip links for main content sections

### Screen Reader Support
- Semantic HTML structure with proper headings
- ARIA labels for interactive elements
- Alt text for any visual examples

### Color and Contrast
- High contrast code syntax highlighting
- Accessible color choices for component variants
- Support for system dark/light mode preferences

## Performance Optimization

### Code Splitting
- Lazy load component examples as user scrolls
- Split code highlighting library to reduce initial bundle
- Use Fresh 2.0 islands for interactive components only

### Caching Strategy
- Static generation for component documentation
- Browser caching for code examples
- Efficient re-rendering with Preact signals

### Bundle Optimization
- Tree shake unused UI library components
- Minimize CSS with Tailwind purging
- Optimize images and static assets