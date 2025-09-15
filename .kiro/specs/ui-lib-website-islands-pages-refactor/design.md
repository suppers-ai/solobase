# Design Document

## Overview

The UI Library Website Islands and Pages refactor will transform the current islands and pages sections from showcasing pre-built islands to educational resources that teach developers how to create interactive islands from ui-lib components and build complete pages. This refactor addresses the architectural change where ui-lib no longer contains islands, instead focusing on providing static components that can be enhanced with client-side interactivity.

## Architecture

### Current State Analysis
- **Islands Data**: Currently contains hardcoded island definitions that reference non-existent components
- **Pages Data**: Contains comprehensive page examples but focuses on routes rather than component composition
- **Routes**: `/islands` and `/pages` routes exist but need complete restructuring

### New Architecture Approach
- **Educational Focus**: Transform from showcase to tutorial/documentation
- **Practical Examples**: Provide working code examples that developers can copy and adapt
- **Progressive Complexity**: Structure content from basic to advanced patterns
- **Component-Centric**: Show how to enhance ui-lib components with interactivity

## Components and Interfaces

### Islands Section Redesign

#### Island Pattern Categories
```typescript
interface IslandPattern {
  id: string;
  name: string;
  description: string;
  complexity: 'basic' | 'medium' | 'advanced';
  baseComponent: string; // ui-lib component used as base
  interactivityType: string; // e.g., "state-management", "form-handling", "api-integration"
  example: {
    staticComponent: string; // JSX for static version
    islandComponent: string; // JSX for island version
    explanation: string;
  };
  codeFiles: {
    filename: string;
    content: string;
    language: 'tsx' | 'ts';
  }[];
  hooks: string[]; // React hooks used
  dependencies?: string[]; // Additional dependencies if needed
}
```

#### Basic Patterns
1. **Button with State** - Convert static Button to interactive with click handling
2. **Theme Toggle** - ThemeController component with localStorage persistence
3. **Form Input** - Input components with validation and state
4. **Modal Trigger** - Button + Modal combination with open/close state

#### Medium Patterns
1. **Search Interface** - SearchButton + SearchModal with filtering logic
2. **Data Table** - Table component with sorting, filtering, pagination
3. **Form Wizard** - Multi-step form using Card and Button components
4. **Shopping Cart** - Badge + Dropdown with cart state management

#### Advanced Patterns
1. **Real-time Chat** - ChatBubble components with WebSocket integration
2. **Dashboard** - Complex layout with multiple interactive components
3. **File Upload** - FileInput with progress, preview, and error handling
4. **Infinite Scroll** - Card grid with dynamic loading

### Pages Section Redesign

#### Page Template Categories
```typescript
interface PageTemplate {
  id: string;
  name: string;
  description: string;
  category: 'landing' | 'dashboard' | 'form' | 'admin' | 'auth' | 'content';
  components: string[]; // ui-lib components used
  layout: {
    structure: string; // Description of layout structure
    responsive: boolean;
    sections: string[]; // Header, main, footer, etc.
  };
  features: string[]; // Key features demonstrated
  codeFiles: {
    filename: string;
    content: string;
    language: 'tsx' | 'ts' | 'css';
  }[];
  liveExample?: string; // URL to working example if available
}
```

#### Page Categories

**Landing Pages**
1. **Hero Landing** - HeroSection + features grid using Card components
2. **Product Showcase** - Carousel + Card + Button for product display
3. **Service Landing** - Stats + testimonials using display components

**Dashboard Pages**
1. **Admin Dashboard** - Sidebar + Stats + Table + Charts
2. **User Dashboard** - Profile + recent activity + quick actions
3. **Analytics Dashboard** - Complex data visualization with interactive elements

**Form Pages**
1. **Contact Form** - Comprehensive form with validation
2. **Registration** - Multi-step signup process
3. **Profile Settings** - User profile management interface

**Authentication Pages**
1. **Login Page** - Clean login interface with OAuth options
2. **Signup Page** - Registration with terms and validation
3. **Password Reset** - Forgot password flow

## Data Models

### Island Examples Data Structure
```typescript
interface IslandExampleData {
  patterns: {
    basic: IslandPattern[];
    medium: IslandPattern[];
    advanced: IslandPattern[];
  };
  categories: {
    name: string;
    description: string;
    patterns: string[]; // pattern IDs
  }[];
  tutorials: {
    title: string;
    description: string;
    steps: {
      title: string;
      content: string;
      code?: string;
    }[];
  }[];
}
```

### Page Templates Data Structure
```typescript
interface PageTemplateData {
  templates: {
    landing: PageTemplate[];
    dashboard: PageTemplate[];
    form: PageTemplate[];
    admin: PageTemplate[];
    auth: PageTemplate[];
    content: PageTemplate[];
  };
  guides: {
    title: string;
    description: string;
    sections: {
      title: string;
      content: string;
      examples?: string[];
    }[];
  }[];
}
```

## Error Handling

### Code Example Validation
- Validate all code examples compile correctly
- Ensure imports reference actual ui-lib components
- Test that examples work in Fresh environment

### Interactive Example Error Boundaries
- Wrap live examples in error boundaries
- Provide fallback UI when examples fail
- Display helpful error messages for debugging

### Copy-to-Clipboard Error Handling
- Handle clipboard API failures gracefully
- Provide visual feedback for copy operations
- Fall back to text selection if needed

## Testing Strategy

### Example Code Testing
```typescript
// Test that island examples compile and work
test("Button with State island example works", () => {
  const { getByText, getByRole } = render(<ButtonWithStateExample />);
  
  const button = getByRole('button');
  expect(button).toHaveTextContent('Click me');
  
  fireEvent.click(button);
  expect(button).toHaveTextContent('Clicked!');
});

// Test page template compilation
test("Landing page template renders correctly", () => {
  const { container } = render(<LandingPageTemplate />);
  
  expect(container.querySelector('header')).toBeInTheDocument();
  expect(container.querySelector('main')).toBeInTheDocument();
  expect(container.querySelector('footer')).toBeInTheDocument();
});
```

### Content Validation Testing
- Verify all referenced ui-lib components exist
- Test that code examples are syntactically correct
- Validate that complexity levels are properly categorized

### User Experience Testing
- Test navigation between examples
- Verify copy-to-clipboard functionality
- Test responsive behavior on different devices

## Implementation Approach

### Phase 1: Data Structure Refactor
1. Replace current islands.ts with new IslandPattern data structure
2. Update pages.ts to focus on PageTemplate examples
3. Create utility functions for filtering and categorization

### Phase 2: Islands Section Implementation
1. Create new /islands route with educational content
2. Implement pattern showcase with complexity filtering
3. Add interactive code examples with copy functionality
4. Create tutorial sections for common patterns

### Phase 3: Pages Section Implementation
1. Redesign /pages route to show page templates
2. Implement category-based browsing
3. Add complete page examples with responsive previews
4. Create composition guides and best practices

### Phase 4: Integration and Polish
1. Update navigation and cross-linking
2. Add search functionality across patterns and templates
3. Implement responsive design improvements
4. Add accessibility enhancements

## User Interface Design

### Islands Page Layout
```
Header: "Creating Interactive Islands"
├── Introduction: What are islands and when to use them
├── Pattern Categories: Basic | Medium | Advanced
├── Pattern Grid: Cards showing each pattern with preview
└── Tutorial Section: Step-by-step guides

Pattern Detail View:
├── Pattern Description
├── Before/After: Static vs Interactive comparison
├── Code Tabs: Component files with syntax highlighting
├── Live Example: Working demonstration
└── Related Patterns: Links to similar examples
```

### Pages Page Layout
```
Header: "Building Complete Pages"
├── Introduction: Page composition with ui-lib components
├── Template Categories: Landing | Dashboard | Form | Admin | Auth
├── Template Grid: Screenshots with component breakdown
└── Composition Guide: Best practices and patterns

Template Detail View:
├── Template Overview: Description and features
├── Component Breakdown: Which ui-lib components are used
├── Code Files: Complete implementation files
├── Responsive Preview: Different screen size views
└── Customization Guide: How to adapt the template
```

## Responsive Design

### Mobile Layout (< 768px)
- Single column layout for pattern/template grids
- Collapsible code sections to save space
- Touch-friendly navigation and interactions
- Simplified previews optimized for small screens

### Tablet Layout (768px - 1024px)
- Two-column grid for patterns/templates
- Side-by-side code and preview where space allows
- Improved navigation with more visible categories

### Desktop Layout (> 1024px)
- Multi-column grids for efficient space usage
- Larger code examples with better readability
- Split-screen views for before/after comparisons
- Enhanced filtering and search capabilities

## Accessibility Considerations

### Keyboard Navigation
- All interactive elements accessible via keyboard
- Proper tab order through examples and code
- Skip links for main content sections

### Screen Reader Support
- Semantic HTML structure with proper headings
- ARIA labels for code examples and interactive elements
- Alt text for any visual diagrams or screenshots

### Code Accessibility
- High contrast syntax highlighting
- Scalable text in code examples
- Screen reader friendly code structure

## Performance Optimization

### Code Splitting
- Lazy load pattern examples as user navigates
- Split syntax highlighting library to reduce bundle
- Use Fresh islands only for truly interactive examples

### Caching Strategy
- Static generation for documentation content
- Browser caching for code examples and assets
- Efficient re-rendering with minimal state updates

### Bundle Optimization
- Tree shake unused ui-lib components in examples
- Minimize CSS with Tailwind purging
- Optimize any images or media assets used in examples