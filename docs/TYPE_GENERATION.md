# Type Generation from GORM Models

This document describes the automated TypeScript type generation system that creates type definitions from GORM models to ensure type safety between the backend and frontend.

## Overview

The type generation system automatically extracts struct definitions from Go files containing GORM models and generates corresponding TypeScript interfaces. This prevents type mismatches and ensures that the frontend types stay in sync with the database schema.

## Architecture

```
scripts/generate-types.go         # Type generation script
    ↓ reads
GORM Model Files (.go)           # Source of truth
    ↓ generates
sdk/typescript/src/types/database/index.ts  # Auto-generated types
```

## Usage

### Generate Types Manually

You can generate types using any of these methods:

```bash
# Using Make
make generate-types

# Using npm (from SDK directory)
cd sdk/typescript
npm run generate:types

# Using Go directly
go run scripts/generate-types.go
```

### Automatic Generation

Types are automatically regenerated when building the TypeScript SDK:

```bash
cd sdk/typescript
npm run build  # This runs generate:types as a prebuild step
```

## How It Works

1. **Model Discovery**: The script scans predefined paths for Go files containing GORM models:
   - `packages/auth/models.go`
   - `packages/storage/models.go`
   - `internal/iam/models.go`
   - `extensions/official/*/models.go`
   - And more...

2. **AST Parsing**: Uses Go's AST parser to extract struct definitions with their fields, types, and tags.

3. **Type Mapping**: Converts Go types to TypeScript equivalents:
   - `string` → `string`
   - `int`, `uint` → `number`
   - `bool` → `boolean`
   - `time.Time` → `string | Date`
   - `uuid.UUID` → `string`
   - Pointers (`*Type`) → `Type | null`
   - Arrays (`[]Type`) → `Type[]`
   - Maps → `Record<K, V>`

4. **Interface Generation**: Creates TypeScript interfaces with:
   - Proper field names from JSON tags
   - Optional fields marked with `?`
   - Comments preserved as JSDoc
   - Package-prefixed names to avoid conflicts

## Generated File Structure

The generated file (`sdk/typescript/src/types/database/index.ts`) contains:

1. **Header**: Warning and timestamp
2. **Interfaces**: Grouped by package
3. **Helper Types**: Common type aliases
4. **Table Names**: Constants for database table names

Example output:

```typescript
// Auto-generated from GORM models - DO NOT EDIT MANUALLY
// Generated at: 2025-01-15T10:00:00Z

export interface AuthUser {
  id: string;
  email: string;
  username?: string;
  confirmed: boolean;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface IAMRole {
  id: string;
  name: string;
  display_name: string;
  is_system: boolean;
  metadata?: Record<string, any>;
}
```

## Adding New Models

To include new GORM models in type generation:

1. Add the model file path to the `modelPaths` array in `scripts/generate-types.go`
2. Ensure your structs have proper JSON tags
3. Run `make generate-types`

## Best Practices

1. **JSON Tags**: Always include JSON tags on exported fields
2. **Comments**: Add field comments for documentation
3. **Naming**: Use clear, consistent struct names
4. **Nullability**: Use pointers for nullable database fields
5. **Regeneration**: Run type generation after model changes

## Type Usage in Frontend

The generated types are automatically exported from the main types file:

```typescript
import { AuthUser, IAMRole, StorageObject } from '@solobase/sdk/types';

// Types are now available with full IntelliSense
const user: AuthUser = {
  id: '123',
  email: 'user@example.com',
  confirmed: true,
  created_at: new Date(),
  updated_at: new Date()
};
```

## Troubleshooting

### Types Not Updating

1. Ensure the model file is included in `modelPaths`
2. Check that structs have exported fields (capitalized)
3. Verify JSON tags are present
4. Run generation manually to see any errors

### Compilation Errors

1. Check for TypeScript syntax errors in generated file
2. Ensure all Go types have mappings defined
3. Look for special characters in field names

### Missing Fields

1. Only exported fields are included
2. Fields with `json:"-"` tag are excluded
3. Embedded structs are currently skipped

## Future Enhancements

- [ ] Support for embedded structs
- [ ] Extract validation rules from tags
- [ ] Generate API client methods
- [ ] Support for custom type mappings
- [ ] Watch mode for development
- [ ] Generate types for other languages (Python, Java)