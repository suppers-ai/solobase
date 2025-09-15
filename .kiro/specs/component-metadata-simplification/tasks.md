# Implementation Plan

- [x] 1. Update ComponentExample interface in types.ts
  - Remove `code`, `staticRender`, and `showCode` properties from ComponentExample interface
  - Make `props` property required and support both single object and array of objects
  - Update JSDoc comments to reflect the simplified approach
  - _Requirements: 1.1, 1.4_

- [x] 2. Create propsToJSX utility function
  - Implement function that converts props object to readable JSX string
  - Handle different prop types: strings, booleans, numbers, objects, arrays, functions
  - Format JSX attributes with proper spacing and quotes
  - Handle children prop specially to generate proper JSX structure
  - Add proper indentation for multi-line JSX
  - _Requirements: 2.2, 2.3_

- [x] 3. Simplify component route handler
  - Remove complex JSX parsing logic (parseJSXExample and parseComplexProps functions)
  - Remove staticRender handling code
  - Remove showCode conditional logic
  - Update example rendering to use only props-based approach
  - Integrate propsToJSX function for automatic code generation
  - _Requirements: 3.1, 3.2, 3.4_

- [x] 4. Update Button component metadata
  - Convert existing Button examples from code-based to props-based format
  - Remove code and showCode properties from all examples
  - Extract props from existing JSX code strings
  - Test that visual output remains the same
  - _Requirements: 4.2, 4.3_

- [ ] 5. Update Navbar component metadata
  - Convert existing Navbar examples from mixed format to props-only format
  - Handle complex JSX children props properly
  - Remove code and showCode properties
  - Verify dropdown and responsive examples work correctly
  - _Requirements: 4.2, 4.3_

- [x] 6. Create migration script for remaining components
  - Write script to identify all component metadata files
  - Parse existing examples and convert code strings to props objects where possible
  - Handle edge cases and complex JSX structures
  - Generate report of components that need manual migration
  - _Requirements: 4.1, 4.2_

- [x] 7. Test propsToJSX function with various prop types
  - Write unit tests for string, boolean, number prop formatting
  - Test object and array prop serialization
  - Test function prop handling (should show simplified representation)
  - Test JSX children prop handling
  - Test multi-component array rendering
  - _Requirements: 2.3, 3.3_

- [x] 8. Test simplified route handler
  - Write integration tests for component page rendering
  - Test single component examples
  - Test multi-component examples (array of props)
  - Test interactive vs static component selection
  - Verify code generation accuracy
  - _Requirements: 3.1, 3.2_

- [x] 9. Update remaining component metadata files
  - Apply migration script results to all component metadata files
  - Manually fix any components that couldn't be automatically migrated
  - Verify all examples render correctly
  - Remove any remaining code, staticRender, showCode properties
  - _Requirements: 4.1, 4.4_

- [x] 10. Clean up and optimize
  - Remove unused parsing functions from route handler
  - Remove old properties from ComponentExample interface completely
  - Update any remaining references to old metadata structure
  - Add JSDoc documentation for new simplified approach
  - _Requirements: 3.4, 1.4_