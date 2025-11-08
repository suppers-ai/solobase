/**
 * Static assets module for Solobase SDK
 * Provides paths to bundled static assets like logos and icons
 */

// Asset paths relative to the package root
export const SOLOBASE_ASSETS = {
  logo: '@solobase/sdk/static/logo.png',
  logoLong: '@solobase/sdk/static/logo_long.png',
  favicon: '@solobase/sdk/static/favicon.ico',
} as const;

// Helper to get the absolute path to assets
export function getSolobaseAssetPath(asset: keyof typeof SOLOBASE_ASSETS): string {
  return `/node_modules/${SOLOBASE_ASSETS[asset]}`;
}

// Export for convenience
export const solobaseAssets = SOLOBASE_ASSETS;