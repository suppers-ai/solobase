/**
 * Unified Field Definition Types
 * Matches the Go FieldDefinition structure
 */

// Field constraints matching Go FieldConstraints struct
export interface FieldConstraints {
  required?: boolean;
  min?: number;
  max?: number;
  minLength?: number;
  maxLength?: number;
  pattern?: string;
  options?: string[];
  default?: any;
  placeholder?: string;
  rows?: number;
  step?: number;
  editableByUser?: boolean;
}

// Unified field definition matching Go FieldDefinition struct
export interface FieldDefinition {
  id: string;                    // Field ID (for filter fields: "filter_text_1", etc.)
  name: string;                  // Display name
  type: string;                  // Field type: text, numeric, boolean, enum, color, etc.
  required?: boolean;
  description?: string;
  section?: string;              // Section/tab for UI organization
  order?: number;                // Display order within section
  constraints?: FieldConstraints; // All validation constraints
}

// Helper type for filter field IDs
export type FilterFieldID =
  | `filter_text_${1 | 2 | 3 | 4 | 5}`
  | `filter_numeric_${1 | 2 | 3 | 4 | 5}`
  | `filter_boolean_${1 | 2 | 3 | 4 | 5}`
  | `filter_enum_${1 | 2 | 3 | 4 | 5}`
  | `filter_location_${1 | 2 | 3 | 4 | 5}`;

// Helper function to check if a field ID is a filter field
export function isFilterField(fieldId: string): boolean {
  return /^filter_(text|numeric|boolean|enum|location)_[1-5]$/.test(fieldId);
}

// Helper function to get the type of a filter field from its ID
export function getFilterFieldType(fieldId: string): string | null {
  const match = fieldId.match(/^filter_(text|numeric|boolean|enum|location)_[1-5]$/);
  if (match) {
    const type = match[1];
    return type === 'enum' ? 'select' : type;
  }
  return null;
}