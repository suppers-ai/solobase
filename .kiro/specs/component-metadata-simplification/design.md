# Design Document

## Overview

This design outlines the simplification of the component metadata system by removing complex rendering approaches and standardizing on a props-based system. The goal is to make component documentation easier to maintain while providing consistent, automatic code generation.

## Architecture

### Current State Analysis

The current system has multiple rendering approaches:
- `staticRender`: Direct JSX rendering
- `code`: Raw HTML/JSX string with complex parsing
- `showCode`: Boolean flag to control code visibility
- `props`: Object-based props (sometimes used, sometimes not)

This creates confusion and maintenance overhead in the route handler with complex parsing logic.

### Target State

The simplified system will have:
- **Single rendering approach**: Props-based only
- **Automatic code generation**: JSX code generated from props
- **Consistent presentation**: All examples show component + code
- **Simplified metadata**: Only `title`, `description`, and `props` needed

## Components and Interfaces

### Updated ComponentExample Interface

```typescript
export interface ComponentExample {
  title: string;
  description: string;
  props: Record<string, any> | Array<Record<string, any>>; // Single or multiple component instances
  interactive?: boolean; // Whether to use interactive version of component
}
```

### Removed Properties

- `code: string` - No longer needed, generated automatically
- `staticRender: ComponentChildren` - Replaced by props-based rendering
- `showCode?: boolean` - Code always shown by default

### Code Generation System

The system will include a new `propsToJSX` utility function that converts props objects back to readable JSX syntax:

```typescript
function propsToJSX(componentName: string, props: Record<string, any>, children?: string): string {
  // Convert props object to JSX attribute string
  // Handle different prop types (string, boolean, number, object, function)
  // Generate clean, readable JSX code
}
```

## Data Models

### Example Data Structure

**Before (complex):**
```typescript
{
  title: "Basic Colors",
  description: "Standard button colors and variants",
  code: `<div class="flex gap-2">
    <Button>Default</Button>
    <Button color="primary">Primary</Button>
  </div>`,
  showCode: true,
}
```

**After (simplified):**
```typescript
{
  title: "Basic Colors", 
  description: "Standard button colors and variants",
  props: [
    { children: "Default" },
    { color: "primary", children: "Primary" }
  ]
}
```

### Props Handling

The system will support:
- **Single component**: `props: { color: "primary", children: "Click me" }`
- **Multiple components**: `props: [{ color: "primary" }, { color: "secondary" }]`
- **Complex props**: Objects, arrays, functions (with proper JSX formatting)

## Error Handling

### Runtime Error Handling

- Props validation using existing schema system
- Fallback rendering for invalid props
- Clear error messages in development mode
- Graceful degradation in production

## Testing Strategy

### Unit Tests

- `propsToJSX` function with various prop types
- Component rendering with different prop configurations
- Error handling for invalid props
- Migration utility functions

### Integration Tests

- Full component page rendering
- Code generation accuracy
- Interactive vs static component selection
- Schema validation integration

### Migration Testing

- Before/after visual comparison
- Automated migration script validation
- Component functionality preservation
- Performance impact assessment

## Implementation Plan

### Phase 1: Core Infrastructure

1. Update `ComponentExample` interface
2. Implement `propsToJSX` utility function
3. Create migration utilities
4. Update route handler to use props-only approach

### Phase 2: Component Migration

1. Identify all components using old metadata structure
2. Create automated migration script where possible
3. Manually migrate complex examples
4. Update component metadata files

### Phase 3: Cleanup

1. Remove old parsing logic from route handler
2. Remove unused properties from interfaces
3. Update documentation and examples
4. Performance optimization

## Code Generation Details

### JSX Attribute Formatting

The `propsToJSX` function will handle:

- **Strings**: `title="Hello World"`
- **Booleans**: `disabled` (true) or omitted (false)
- **Numbers**: `count={42}`
- **Objects**: `style={{ color: "red" }}`
- **Arrays**: `items={["a", "b", "c"]}`
- **Functions**: `onClick={() => {}}` (simplified representation)
- **JSX Elements**: `icon={<Icon />}` (serialized appropriately)

### Multi-Component Examples

For arrays of props, the system will:
1. Wrap multiple components in a container div
2. Generate individual JSX for each component
3. Apply consistent spacing and layout
4. Maintain proper indentation in generated code

### Code Formatting

Generated JSX will be:
- Properly indented
- Consistently formatted
- Readable and copy-pasteable
- Syntax highlighted in the UI

## Benefits

### For Developers

- **Simpler maintenance**: Only need to define props
- **Consistent approach**: Same pattern for all components
- **Automatic code**: No manual JSX string writing
- **Type safety**: Props validated against schemas

### For Users

- **Consistent experience**: All examples look the same
- **Always up-to-date code**: Generated from actual props
- **Copy-pasteable examples**: Clean, working JSX
- **Better understanding**: Clear prop-to-output relationship

### For System

- **Reduced complexity**: Simpler route handler
- **Better performance**: No complex parsing
- **Easier testing**: Predictable behavior
- **Future-proof**: Extensible design