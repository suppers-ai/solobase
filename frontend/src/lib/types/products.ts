/**
 * Product Custom Field Types and Constraints
 * A clean, maintainable structure for defining custom field configurations
 */

import type { FieldDefinition } from './field-definition';

// ============================================
// BASE INTERFACES
// ============================================

export type CustomFieldType =
  | 'text'
  | 'textarea'
  | 'number'
  | 'boolean'
  | 'select'
  | 'color'
  | 'date'
  | 'datetime'
  | 'email'
  | 'url'
  | 'range'
  | 'upload'
  | 'file'
  | 'image'
  | 'json'
  | 'markdown';

export interface BaseCustomFieldDefinition {
  // Unique identifier for the custom field
  id: string;

  // Display name for the field
  name: string;

  // Field type
  type: CustomFieldType;

  // Whether field is required
  required?: boolean;

  // Field description/help text
  description?: string;

  // Default value (type depends on field type)
  default?: any;

  // UI placeholder text
  placeholder?: string;

  // Section/group this field belongs to
  section?: string;

  // Display order
  order?: number;

  // Whether field is disabled
  disabled?: boolean;

  // Whether field is hidden
  hidden?: boolean;

  // Custom validation message
  validationMessage?: string;
}

// ============================================
// CONSTRAINT INTERFACES
// ============================================

export interface TextConstraints {
  minLength?: number;
  maxLength?: number;
  pattern?: string;
  trim?: boolean;
  lowercase?: boolean;
  uppercase?: boolean;
}

export interface TextareaConstraints extends TextConstraints {
  rows?: number;
  cols?: number;
  resize?: 'none' | 'both' | 'horizontal' | 'vertical';
}

export interface NumericConstraints {
  min?: number;
  max?: number;
  step?: number;
  precision?: number;
  thousandsSeparator?: boolean;
  allowNegative?: boolean;
}

export interface RangeConstraints extends NumericConstraints {
  showValue?: boolean;
  showLabels?: boolean;
  marks?: Array<{ value: number; label?: string }>;
}

export interface SelectConstraints {
  options: Array<string | { value: string; label: string; disabled?: boolean }>;
  multiple?: boolean;
  searchable?: boolean;
  clearable?: boolean;
  maxSelections?: number;
}

export interface DateConstraints {
  minDate?: string | Date;
  maxDate?: string | Date;
  disabledDates?: Array<string | Date>;
  format?: string;
  includeTime?: boolean;
  timeFormat?: '12h' | '24h';
}

export interface ColorConstraints {
  format?: 'hex' | 'rgb' | 'hsl';
  showAlpha?: boolean;
  presets?: string[];
  showInput?: boolean;
}

export interface FileConstraints {
  accept?: string; // e.g., 'image/*', '.pdf,.doc'
  maxSize?: number; // in bytes
  minSize?: number; // in bytes
  multiple?: boolean;
  maxFiles?: number;
  uploadUrl?: string;
  storageType?: 'local' | 'cloud' | 'base64';
}

export interface ImageConstraints extends FileConstraints {
  minWidth?: number;
  maxWidth?: number;
  minHeight?: number;
  maxHeight?: number;
  aspectRatio?: string; // e.g., '16:9', '1:1'
  preview?: boolean;
  crop?: boolean;
  resize?: boolean;
}

export interface JsonConstraints {
  schema?: object; // JSON Schema for validation
  prettify?: boolean;
  collapsible?: boolean;
  maxDepth?: number;
}

export interface MarkdownConstraints {
  maxLength?: number;
  toolbar?: boolean;
  preview?: boolean;
  allowHtml?: boolean;
  sanitize?: boolean;
}

// ============================================
// FIELD TYPE DEFINITIONS WITH CONSTRAINTS
// ============================================

export interface TextFieldDefinition extends BaseCustomFieldDefinition {
  type: 'text';
  constraints?: TextConstraints;
}

export interface TextareaFieldDefinition extends BaseCustomFieldDefinition {
  type: 'textarea';
  constraints?: TextareaConstraints;
}

export interface NumericFieldDefinition extends BaseCustomFieldDefinition {
  type: 'number';
  constraints?: NumericConstraints;
}

export interface RangeFieldDefinition extends BaseCustomFieldDefinition {
  type: 'range';
  constraints?: RangeConstraints;
}

export interface BooleanFieldDefinition extends BaseCustomFieldDefinition {
  type: 'boolean';
  constraints?: {
    trueLabel?: string;
    falseLabel?: string;
    style?: 'checkbox' | 'toggle' | 'radio';
  };
}

export interface SelectFieldDefinition extends BaseCustomFieldDefinition {
  type: 'select';
  constraints?: SelectConstraints;
}

export interface DateFieldDefinition extends BaseCustomFieldDefinition {
  type: 'date' | 'datetime';
  constraints?: DateConstraints;
}

export interface ColorFieldDefinition extends BaseCustomFieldDefinition {
  type: 'color';
  constraints?: ColorConstraints;
}

export interface EmailFieldDefinition extends BaseCustomFieldDefinition {
  type: 'email';
  constraints?: TextConstraints & {
    allowMultiple?: boolean;
    separator?: string;
    validateDomain?: boolean;
  };
}

export interface UrlFieldDefinition extends BaseCustomFieldDefinition {
  type: 'url';
  constraints?: TextConstraints & {
    protocols?: string[]; // ['http', 'https', 'ftp']
    requireProtocol?: boolean;
    validateDomain?: boolean;
  };
}

export interface FileFieldDefinition extends BaseCustomFieldDefinition {
  type: 'upload' | 'file';
  constraints?: FileConstraints;
}

export interface ImageFieldDefinition extends BaseCustomFieldDefinition {
  type: 'image';
  constraints?: ImageConstraints;
}

export interface JsonFieldDefinition extends BaseCustomFieldDefinition {
  type: 'json';
  constraints?: JsonConstraints;
}

export interface MarkdownFieldDefinition extends BaseCustomFieldDefinition {
  type: 'markdown';
  constraints?: MarkdownConstraints;
}

// ============================================
// UNION TYPE FOR ALL FIELD DEFINITIONS
// ============================================

export type ProductCustomFieldDefinition =
  | TextFieldDefinition
  | TextareaFieldDefinition
  | NumericFieldDefinition
  | RangeFieldDefinition
  | BooleanFieldDefinition
  | SelectFieldDefinition
  | DateFieldDefinition
  | ColorFieldDefinition
  | EmailFieldDefinition
  | UrlFieldDefinition
  | FileFieldDefinition
  | ImageFieldDefinition
  | JsonFieldDefinition
  | MarkdownFieldDefinition;

// ============================================
// HELPER FUNCTIONS
// ============================================

/**
 * Type guard to check if a field is of a specific type
 */
export function isFieldType<T extends CustomFieldType>(
  field: ProductCustomFieldDefinition,
  type: T
): field is Extract<ProductCustomFieldDefinition, { type: T }> {
  return field.type === type;
}

/**
 * Get default constraints for a field type
 */
export function getDefaultConstraints(type: CustomFieldType): any {
  switch (type) {
    case 'text':
      return { maxLength: 255 };
    case 'textarea':
      return { maxLength: 5000, rows: 4 };
    case 'number':
      return { min: 0, step: 1 };
    case 'range':
      return { min: 0, max: 100, step: 1, showValue: true };
    case 'select':
      return { options: [], searchable: true };
    case 'date':
    case 'datetime':
      return { format: 'YYYY-MM-DD' };
    case 'color':
      return { format: 'hex', showInput: true };
    case 'upload':
    case 'file':
      return { maxSize: 10485760 }; // 10MB
    case 'image':
      return { accept: 'image/*', maxSize: 5242880, preview: true }; // 5MB
    default:
      return {};
  }
}

/**
 * Validate field value against constraints
 */
export function validateFieldValue(
  field: ProductCustomFieldDefinition,
  value: any
): { valid: boolean; error?: string } {
  // Required field check
  if (field.required && (value === undefined || value === null || value === '')) {
    return { valid: false, error: `${field.name} is required` };
  }

  // Skip validation if not required and empty
  if (!field.required && (value === undefined || value === null || value === '')) {
    return { valid: true };
  }

  // Type-specific validation
  switch (field.type) {
    case 'text':
    case 'email':
    case 'url': {
      const constraints = (field as TextFieldDefinition).constraints;
      if (constraints?.minLength && value.length < constraints.minLength) {
        return {
          valid: false,
          error: `${field.name} must be at least ${constraints.minLength} characters`
        };
      }
      if (constraints?.maxLength && value.length > constraints.maxLength) {
        return {
          valid: false,
          error: `${field.name} must be no more than ${constraints.maxLength} characters`
        };
      }
      if (constraints?.pattern) {
        const regex = new RegExp(constraints.pattern);
        if (!regex.test(value)) {
          return {
            valid: false,
            error: field.validationMessage || `${field.name} format is invalid`
          };
        }
      }
      break;
    }

    case 'number': {
      const constraints = (field as NumericFieldDefinition).constraints;
      const numValue = Number(value);
      if (isNaN(numValue)) {
        return { valid: false, error: `${field.name} must be a number` };
      }
      if (constraints?.min !== undefined && numValue < constraints.min) {
        return {
          valid: false,
          error: `${field.name} must be at least ${constraints.min}`
        };
      }
      if (constraints?.max !== undefined && numValue > constraints.max) {
        return {
          valid: false,
          error: `${field.name} must be no more than ${constraints.max}`
        };
      }
      break;
    }

    case 'select': {
      const constraints = (field as SelectFieldDefinition).constraints;
      if (!constraints?.options) break;

      const validOptions = constraints.options.map(opt =>
        typeof opt === 'string' ? opt : opt.value
      );

      if (constraints.multiple) {
        if (!Array.isArray(value)) {
          return { valid: false, error: `${field.name} must be an array` };
        }
        const invalidValues = value.filter(v => !validOptions.includes(v));
        if (invalidValues.length > 0) {
          return { valid: false, error: `Invalid options selected for ${field.name}` };
        }
      } else {
        if (!validOptions.includes(value)) {
          return { valid: false, error: `Invalid option selected for ${field.name}` };
        }
      }
      break;
    }

    // Add more validation as needed
  }

  return { valid: true };
}

// ============================================
// PRODUCT TEMPLATE INTERFACES
// ============================================

export interface ProductTemplate {
  id: string | number;
  name: string;
  displayName: string;
  description?: string;
  category?: string;
  icon?: string;
  filterFieldsSchema: FieldDefinition[];  // Filter fields (mapped to DB columns)
  customFieldsSchema: FieldDefinition[];  // Custom fields (stored as JSON)
  billingMode?: string;
  billingType?: string;
  status?: string;
}

