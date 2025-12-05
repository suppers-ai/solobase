// Export all product-related types
export * from './products';
export * from './field-definition';

// Re-export commonly used types for convenience
export type {
  CustomFieldType,
  ProductCustomFieldDefinition,
  ProductTemplate,
  TextConstraints,
  NumericConstraints,
  SelectConstraints,
  FileConstraints,
  ImageConstraints
} from './products';

export type {
  FieldDefinition,
  FieldConstraints,
  FilterFieldID
} from './field-definition';