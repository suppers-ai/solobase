import { SolobaseClient } from '../src';

// Initialize the client
const solobase = new SolobaseClient({
  url: 'http://localhost:8090',
  // apiKey: 'your-api-key', // Optional: Use API key for server-side usage
});

async function main() {
  try {
    // ========================================
    // Authentication Examples
    // ========================================
    
    // Sign up a new user
    const { user, tokens } = await solobase.auth.signUp({
      email: 'user@example.com',
      password: 'SecurePassword123!',
      metadata: {
        name: 'John Doe',
        company: 'Example Corp',
      },
    });
    console.log('User created:', user);

    // Sign in
    await solobase.auth.signIn({
      email: 'user@example.com',
      password: 'SecurePassword123!',
    });

    // Get current user
    const currentUser = await solobase.auth.getUser();
    console.log('Current user:', currentUser);

    // ========================================
    // Storage Examples
    // ========================================
    
    // Create a bucket
    const bucket = await solobase.storage.createBucket('my-files', false);
    console.log('Bucket created:', bucket);

    // Upload a file (in Node.js)
    const buffer = Buffer.from('Hello, World!', 'utf-8');
    const uploadedFile = await solobase.storage.upload(
      'my-files',
      buffer,
      'hello.txt',
      {
        contentType: 'text/plain',
        metadata: { description: 'Test file' },
        onProgress: (progress) => console.log(`Upload progress: ${progress}%`),
      }
    );
    console.log('File uploaded:', uploadedFile);

    // List files in bucket
    const files = await solobase.storage.list('my-files', {
      limit: 10,
      offset: 0,
    });
    console.log('Files in bucket:', files);

    // Get signed URL for file
    const signedUrl = await solobase.storage.getSignedUrl('my-files', 'hello.txt');
    console.log('Signed URL:', signedUrl);

    // Get storage stats
    const stats = await solobase.storage.getStats();
    console.log('Storage stats:', stats);

    // ========================================
    // Database/Collections Examples
    // ========================================
    
    // Create a collection
    const collection = await solobase.database.createCollection('products', {
      name: { type: 'string', required: true },
      price: { type: 'number', required: true },
      description: { type: 'string' },
      in_stock: { type: 'boolean', default: true },
    });
    console.log('Collection created:', collection);

    // Create a record
    const product = await solobase.database.create({
      collection: 'products',
      data: {
        name: 'Laptop',
        price: 999.99,
        description: 'High-performance laptop',
        in_stock: true,
      },
    });
    console.log('Product created:', product);

    // Query records with builder
    const expensiveProducts = await solobase.database
      .from('products')
      .where('price', '>', 500)
      .orderBy('price', 'desc')
      .limit(5)
      .execute();
    console.log('Expensive products:', expensiveProducts);

    // Update a record
    const updatedProduct = await solobase.database.update({
      collection: 'products',
      id: product.id,
      data: { price: 899.99 },
    });
    console.log('Product updated:', updatedProduct);

    // ========================================
    // Extensions Examples
    // ========================================
    
    // List available extensions
    const extensions = await solobase.extensions.list();
    console.log('Available extensions:', extensions);

    // Enable CloudStorage extension
    await solobase.extensions.enable('cloudstorage', {
      defaultStorageLimit: 10737418240, // 10GB
      enableSharing: true,
      enableQuotas: true,
    });

    // CloudStorage Extension Usage
    if (uploadedFile) {
      // Share a file
      const share = await solobase.cloudStorage.share(uploadedFile.id, {
        email: 'friend@example.com',
        permissions: 'view',
        expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(), // 7 days
      });
      console.log('File shared:', share);

      // Get quota info
      const quota = await solobase.cloudStorage.getQuota();
      console.log('Storage quota:', quota);

      // Get access logs
      const accessLogs = await solobase.cloudStorage.getAccessLogs(uploadedFile.id);
      console.log('Access logs:', accessLogs);
    }

    // Products Extension Usage
    // Create a group (e.g., restaurant)
    const group = await solobase.products.createGroup({
      name: 'My Restaurant',
      template_id: 'restaurant_template',
      custom_fields: {
        address: '123 Main St',
        cuisine: 'Italian',
      },
    });

    // Create a product
    const menuItem = await solobase.products.createProduct({
      group_id: group.id,
      name: 'Margherita Pizza',
      template_id: 'menu_item',
      custom_fields: {
        description: 'Fresh mozzarella, tomato, basil',
        category: 'Pizza',
        allergens: ['dairy', 'gluten'],
      },
      pricing_formula: 'base_price + (size_multiplier * base_price)',
    });

    // Calculate price with variables
    const pricing = await solobase.products.calculatePrice(menuItem.id, {
      base_price: 12,
      size_multiplier: 1.5, // Large size
    });
    console.log('Calculated price:', pricing);

    // ========================================
    // Shortcuts Examples
    // ========================================
    
    // Use convenient shortcut methods
    await solobase.upload('my-files', buffer, 'quick-upload.txt');
    const results = await solobase.query('products', { limit: 5 });
    const user = await solobase.getUser();

    // Sign out
    await solobase.signOut();
    
  } catch (error) {
    console.error('Error:', error);
  }
}

// Run the examples
main();