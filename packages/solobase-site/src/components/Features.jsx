export default function Features() {
  const featureCards = [
    {
      title: 'Authentication',
      description: 'Signup, login, JWT tokens, token refresh, API keys, and OAuth (Google, GitHub). Role-based access control with IAM policies.',
      icon: (
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
        </svg>
      ),
    },
    {
      title: 'Database',
      description: 'SQLite out of the box with collections and custom tables. Automatic migrations, full admin query console, and REST API for CRUD.',
      icon: (
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
        </svg>
      ),
    },
    {
      title: 'File Storage',
      description: 'Buckets, file upload, folders, sharing links, and per-user quotas. Local disk or S3-compatible backends with usage tracking.',
      icon: (
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
      ),
    },
    {
      title: 'Products & Payments',
      description: 'Product catalog, groups, purchases, and Stripe checkout with webhooks. Define plans, accept payments, and track subscriptions.',
      icon: (
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z" />
        </svg>
      ),
    },
    {
      title: 'Admin Panel',
      description: 'User management, IAM roles, database browser, request logs, system settings, and block configuration. Full control from one dashboard.',
      icon: (
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      ),
    },
    {
      title: 'User Dashboard',
      description: 'Ready-made user portal with account settings, plan management, deployments, and API key creation. Works out of the box.',
      icon: (
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
        </svg>
      ),
    },
  ];

  return (
    <>
      {/* Built on WAFER Runtime */}
      <section class="py-responsive bg-white" style={{ paddingTop: '4rem' }}>
        <div class="container-responsive">
          <div class="text-center mb-12 sm:mb-16 lg:mb-20">
            <h2 class="text-responsive-lg font-bold text-gray-900 mb-4 sm:mb-6">Built on the WAFER Runtime</h2>
            <p class="text-responsive-md text-gray-600 max-w-4xl mx-auto leading-relaxed">
              Solobase is powered by composable blocks and flows. Each feature — auth, storage, database, products — is a block you can extend, replace, or combine. No vendor lock-in, complete control.
            </p>
          </div>
        </div>
      </section>

      {/* Everything You Need, Built In */}
      <section class="py-responsive bg-gray-50">
        <div class="container-responsive">
          <div class="text-center mb-12 sm:mb-16">
            <h2 class="text-responsive-lg font-bold text-gray-900 mb-4 sm:mb-6" style={{ marginTop: 0 }}>Everything You Need, Built In</h2>
          </div>
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6 max-w-6xl mx-auto">
            {featureCards.map((card) => (
              <div
                key={card.title}
                class="bg-white rounded-xl p-6 shadow-sm hover:shadow-md transition-shadow duration-200"
                style={{ border: '1px solid #e5e7eb' }}
              >
                <div class="mb-4 text-primary-400">
                  {card.icon}
                </div>
                <h3 class="text-lg font-semibold text-gray-900 mb-2">{card.title}</h3>
                <p class="text-sm text-gray-600 leading-relaxed">{card.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Quick Start Section */}
      <section class="py-responsive bg-white">
        <div class="container-responsive">
          <div class="max-w-4xl mx-auto">
            <h2 class="text-responsive-lg font-bold text-gray-900 mb-8 sm:mb-12 text-center" style={{ marginTop: 0 }}>
              Start building in seconds
            </h2>
            <div class="bg-gray-900 rounded-lg shadow-xl">
              <pre class="text-gray-300 overflow-x-auto">
                <code>
                  <span class="text-gray-500"># Download the latest release</span>{'\n'}
                  <span class="text-white">curl -sSL https://github.com/suppers-ai/solobase/releases/latest/download/solobase-linux-amd64.tar.gz | tar xz</span>{'\n'}
                  {'\n'}
                  <span class="text-gray-500"># Set a JWT secret and run</span>{'\n'}
                  <span class="text-white">export JWT_SECRET="your-secret-key-at-least-32-chars"</span>{'\n'}
                  <span class="text-white">./solobase</span>{'\n'}
                  {'\n'}
                  <span class="text-gray-500"># That's it. Auth, database, storage, products, admin panel — all running on :8090</span>
                </code>
              </pre>
            </div>
            <div class="mt-8 sm:mt-10 text-center">
              <p class="text-responsive-md text-gray-600 mb-16">
                No Docker, no Node.js, no complex configuration. A single binary with everything built in.
              </p>
              <div class="flex flex-col sm:flex-row gap-4 justify-center">
                <a
                  href="/docs/quick-start/"
                  class="inline-flex items-center justify-center px-6 py-3 text-white rounded-lg font-medium transition-all duration-200 hover:shadow-lg transform hover:-translate-y-0.5"
                  style={{ background: '#1f2937', fontSize: '1.1rem', minWidth: '180px' }}
                >
                  Get Started
                  <svg class="ml-2 w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                </a>
                <a
                  href="/docs/"
                  class="inline-flex items-center justify-center px-6 py-3 rounded-lg font-medium transition-all duration-200 hover:shadow-lg transform hover:-translate-y-0.5"
                  style={{ background: 'white', border: '2px solid #1f2937', color: '#1f2937', fontSize: '1.1rem', minWidth: '180px' }}
                >
                  Read the documentation
                </a>
              </div>
            </div>
          </div>
        </div>
      </section>
    </>
  );
}
