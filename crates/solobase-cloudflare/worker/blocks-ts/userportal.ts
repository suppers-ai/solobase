// TypeScript-native userportal block handler.
// Returns portal configuration (branding, feature flags) by reading config values.

import type { Message, BlockResult } from '../types';
import type { RuntimeHost } from '../host';

export async function handle(msg: Message, host: RuntimeHost): Promise<BlockResult> {
  const config = {
    logo_url: await getConfig(host, 'LOGO_URL', '/logo.png'),
    app_name: await getConfig(host, 'APP_NAME', 'Solobase'),
    primary_color: await getConfig(host, 'PRIMARY_COLOR', '#6366f1'),
    enable_oauth: await getConfig(host, 'ENABLE_OAUTH', 'false'),
    allow_signup: await getConfig(host, 'ALLOW_SIGNUP', 'true'),
    show_powered_by: true,
    features: {
      files: await getConfig(host, 'FEATURE_FILES', 'true'),
      products: await getConfig(host, 'FEATURE_PRODUCTS', 'true'),
      user_products: await getConfig(host, 'FEATURE_USER_PRODUCTS', 'true'),
      legal_pages: await getConfig(host, 'FEATURE_LEGAL_PAGES', 'true'),
      monitoring: await getConfig(host, 'FEATURE_MONITORING', 'true'),
      deployments: await getConfig(host, 'FEATURE_DEPLOYMENTS', 'true'),
    },
  };

  return {
    action: 'respond',
    response: {
      data: new TextEncoder().encode(JSON.stringify(config)),
      meta: [{ key: 'resp.content_type', value: 'application/json' }],
    },
    message: msg,
  };
}

async function getConfig(host: RuntimeHost, key: string, defaultVal: string): Promise<string> {
  const result = await host.callBlock('wafer-run/config', {
    kind: 'config.get',
    data: new TextEncoder().encode(JSON.stringify({ key })),
    meta: [],
  });
  if (result.action !== 'respond' || !result.response) return defaultVal;
  try {
    const parsed = JSON.parse(new TextDecoder().decode(result.response.data));
    return parsed?.value ?? defaultVal;
  } catch {
    return defaultVal;
  }
}
