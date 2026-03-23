// @ts-nocheck — WASM module imports don't have TS declarations
/**
 * Static WASM module imports for all jco-transpiled blocks.
 *
 * CF Workers require WASM as static imports (no runtime compilation).
 * Each import gives a pre-compiled WebAssembly.Module.
 */

// System (1 core module)
import systemCore from './wasm-blocks/system/system.component.core.wasm';

// Auth (3 core modules)
import authCore from './wasm-blocks/auth/auth.component.core.wasm';
import authCore2 from './wasm-blocks/auth/auth.component.core2.wasm';
import authCore3 from './wasm-blocks/auth/auth.component.core3.wasm';

// Profile (1 core module)
import profileCore from './wasm-blocks/profile/profile.component.core.wasm';

// Userportal (3 core modules)
import userportalCore from './wasm-blocks/userportal/userportal.component.core.wasm';
import userportalCore2 from './wasm-blocks/userportal/userportal.component.core2.wasm';
import userportalCore3 from './wasm-blocks/userportal/userportal.component.core3.wasm';

// Legalpages (3 core modules)
import legalpagesCore from './wasm-blocks/legalpages/legalpages.component.core.wasm';
import legalpagesCore2 from './wasm-blocks/legalpages/legalpages.component.core2.wasm';
import legalpagesCore3 from './wasm-blocks/legalpages/legalpages.component.core3.wasm';

// Admin (3 core modules)
import adminCore from './wasm-blocks/admin/admin.component.core.wasm';
import adminCore2 from './wasm-blocks/admin/admin.component.core2.wasm';
import adminCore3 from './wasm-blocks/admin/admin.component.core3.wasm';

// Files (3 core modules)
import filesCore from './wasm-blocks/files/files.component.core.wasm';
import filesCore2 from './wasm-blocks/files/files.component.core2.wasm';
import filesCore3 from './wasm-blocks/files/files.component.core3.wasm';

// Products (3 core modules)
import productsCore from './wasm-blocks/products/products.component.core.wasm';
import productsCore2 from './wasm-blocks/products/products.component.core2.wasm';
import productsCore3 from './wasm-blocks/products/products.component.core3.wasm';

// Deployments (3 core modules)
import deploymentsCore from './wasm-blocks/deployments/deployments.component.core.wasm';
import deploymentsCore2 from './wasm-blocks/deployments/deployments.component.core2.wasm';
import deploymentsCore3 from './wasm-blocks/deployments/deployments.component.core3.wasm';

// ---------------------------------------------------------------------------

const MODULES: Record<string, Record<string, WebAssembly.Module>> = {
  system: {
    'system.component.core.wasm': systemCore,
  },
  auth: {
    'auth.component.core.wasm': authCore,
    'auth.component.core2.wasm': authCore2,
    'auth.component.core3.wasm': authCore3,
  },
  profile: {
    'profile.component.core.wasm': profileCore,
  },
  userportal: {
    'userportal.component.core.wasm': userportalCore,
    'userportal.component.core2.wasm': userportalCore2,
    'userportal.component.core3.wasm': userportalCore3,
  },
  legalpages: {
    'legalpages.component.core.wasm': legalpagesCore,
    'legalpages.component.core2.wasm': legalpagesCore2,
    'legalpages.component.core3.wasm': legalpagesCore3,
  },
  admin: {
    'admin.component.core.wasm': adminCore,
    'admin.component.core2.wasm': adminCore2,
    'admin.component.core3.wasm': adminCore3,
  },
  files: {
    'files.component.core.wasm': filesCore,
    'files.component.core2.wasm': filesCore2,
    'files.component.core3.wasm': filesCore3,
  },
  products: {
    'products.component.core.wasm': productsCore,
    'products.component.core2.wasm': productsCore2,
    'products.component.core3.wasm': productsCore3,
  },
  deployments: {
    'deployments.component.core.wasm': deploymentsCore,
    'deployments.component.core2.wasm': deploymentsCore2,
    'deployments.component.core3.wasm': deploymentsCore3,
  },
};

/**
 * Get a getCoreModule callback for a specific block.
 * The callback maps jco file names to pre-compiled WebAssembly.Module objects.
 */
export function getCoreModuleFor(blockId: string): (path: string) => WebAssembly.Module {
  const modules = MODULES[blockId];
  if (!modules) throw new Error(`No WASM modules for block: ${blockId}`);
  return (path: string) => {
    const mod = modules[path];
    if (!mod) throw new Error(`WASM module not found: ${path} for block ${blockId}`);
    return mod;
  };
}

/** Check if a block has WASM modules available. */
export function hasWasmBlock(blockId: string): boolean {
  return blockId in MODULES;
}
