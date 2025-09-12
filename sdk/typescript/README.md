# Solobase TypeScript SDK

Official TypeScript SDK for Solobase - a modern backend-as-a-service platform.

## Installation

```bash
npm install @solobase/sdk
# or
yarn add @solobase/sdk
# or
pnpm add @solobase/sdk
```

## Quick Start

```typescript
import { SolobaseClient } from '@solobase/sdk';

// Initialize the client
const solobase = new SolobaseClient({
  url: 'http://localhost:8090',
  apiKey: 'your-api-key', // Optional: for server-side usage
});

// Sign in a user
await solobase.auth.signIn({
  email: 'user@example.com',
  password: 'password123',
});

// Upload a file
const file = await solobase.storage.upload(
  'my-bucket',
  fileData,
  'document.pdf'
);

// Query data
const products = await solobase.database
  .from('products')
  .where('price', '>', 100)
  .limit(10)
  .execute();
```

## Features

### Authentication

```typescript
// Sign up
const { user, tokens } = await solobase.auth.signUp({
  email: 'user@example.com',
  password: 'SecurePassword123!',
  metadata: { name: 'John Doe' },
});

// Sign in
await solobase.auth.signIn({
  email: 'user@example.com',
  password: 'SecurePassword123!',
});

// Sign out
await solobase.signOut();

// Get current user
const user = await solobase.getUser();

// OAuth sign in
const { url } = await solobase.auth.signInWithOAuth('google');
```

### Storage

```typescript
// Create a bucket
await solobase.storage.createBucket('images', true); // public bucket

// Upload file
const file = await solobase.storage.upload(
  'images',
  imageFile,
  'photo.jpg',
  {
    contentType: 'image/jpeg',
    onProgress: (progress) => console.log(`${progress}%`),
  }
);

// Get signed URL
const url = await solobase.storage.getSignedUrl('images', 'photo.jpg');

// List files
const { data } = await solobase.storage.list('images', {
  limit: 20,
  offset: 0,
});

// Delete file
await solobase.storage.delete('images', 'photo.jpg');
```

### Database/Collections

```typescript
// Create a collection
await solobase.database.createCollection('products', {
  name: { type: 'string', required: true },
  price: { type: 'number', required: true },
  in_stock: { type: 'boolean', default: true },
});

// Insert data
const product = await solobase.database.create({
  collection: 'products',
  data: {
    name: 'Laptop',
    price: 999.99,
    in_stock: true,
  },
});

// Query with builder
const results = await solobase.database
  .from('products')
  .where('price', '<', 1000)
  .where('in_stock', '=', true)
  .orderBy('price', 'desc')
  .limit(10)
  .execute();

// Update record
await solobase.database.update({
  collection: 'products',
  id: product.id,
  data: { price: 899.99 },
});

// Delete record
await solobase.database.delete('products', product.id);
```

### Extensions

```typescript
// List extensions
const extensions = await solobase.extensions.list();

// Enable an extension
await solobase.extensions.enable('cloudstorage', {
  defaultStorageLimit: 10737418240, // 10GB
  enableSharing: true,
});

// CloudStorage extension
const share = await solobase.cloudStorage.share(fileId, {
  email: 'friend@example.com',
  permissions: 'view',
  expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
});

const quota = await solobase.cloudStorage.getQuota();

// Products extension
const product = await solobase.products.createProduct({
  group_id: 'group-123',
  name: 'Premium Plan',
  pricing_formula: 'base_price * quantity',
});

const pricing = await solobase.products.calculatePrice(product.id, {
  base_price: 99,
  quantity: 3,
});
```

## API Reference

### Client Initialization

```typescript
new SolobaseClient(config: SolobaseConfig | string)
```

Config options:
- `url`: The Solobase server URL
- `apiKey`: Optional API key for authentication
- `headers`: Additional headers to include
- `timeout`: Request timeout in milliseconds

### Services

- **auth**: Authentication service
- **storage**: File storage service
- **database**: Database/collections service
- **extensions**: Extensions management
- **cloudStorage**: CloudStorage extension methods
- **products**: Products extension methods

### Shortcut Methods

The client provides convenient shortcut methods:

```typescript
// Instead of: solobase.storage.upload(...)
await solobase.upload('bucket', file, 'name.txt');

// Instead of: solobase.database.query(...)
await solobase.query('collection', { limit: 10 });

// Instead of: solobase.auth.getUser()
await solobase.getUser();

// Instead of: solobase.auth.signIn(...)
await solobase.signIn('email', 'password');

// Instead of: solobase.auth.signOut()
await solobase.signOut();
```

## Browser vs Node.js

The SDK works in both browser and Node.js environments:

### Browser
```typescript
const file = document.getElementById('file-input').files[0];
await solobase.storage.upload('bucket', file, file.name);
```

### Node.js
```typescript
import fs from 'fs';

const buffer = fs.readFileSync('./document.pdf');
await solobase.storage.upload('bucket', buffer, 'document.pdf');
```

## Error Handling

```typescript
try {
  await solobase.auth.signIn({
    email: 'user@example.com',
    password: 'wrong-password',
  });
} catch (error) {
  if (error.error?.code === 'INVALID_CREDENTIALS') {
    console.error('Invalid email or password');
  }
}
```

## TypeScript Support

The SDK is written in TypeScript and provides full type definitions:

```typescript
import type { 
  User, 
  StorageObject, 
  AuthTokens,
  Collection 
} from '@solobase/sdk';
```

## License

MIT