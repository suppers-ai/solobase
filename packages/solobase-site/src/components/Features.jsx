export default function Features() {
  return (
    <>
      <section class="py-responsive bg-white" style={{ paddingTop: '4rem' }}>
        <div class="container-responsive">
          <div class="text-center mb-12 sm:mb-16 lg:mb-20">
            <h2 class="text-responsive-lg font-bold text-gray-900 mb-4 sm:mb-6">Built for Independence</h2>
            <p class="text-responsive-md text-gray-600 max-w-4xl mx-auto leading-relaxed">
              Solobase gives you complete control. No vendor lock-in, setup for simplicity or for scale, your choice.
            </p>
          </div>
        </div>
      </section>

      {/* Quick Start Section */}
      <section class="py-responsive bg-gray-50">
        <div class="container-responsive">
          <div class="max-w-4xl mx-auto">
            <h2 class="text-responsive-lg font-bold text-gray-900 mb-8 sm:mb-12 text-center" style={{ marginTop: 0 }}>
              Start building in seconds
            </h2>
            <div class="bg-gray-900 rounded-lg shadow-xl">
              <pre class="text-gray-300 overflow-x-auto">
                <code>
                  <span class="text-gray-500"># Install</span>{'\n'}
                  <span class="text-white">go install github.com/suppers-ai/solobase/cmd/solobase@latest</span>{'\n'}
                  {'\n'}
                  <span class="text-gray-500"># Run</span>{'\n'}
                  <span class="text-white">solobase</span>{'\n'}
                  {'\n'}
                  <span class="text-gray-500"># That's it. Really.</span>
                </code>
              </pre>
            </div>
            <div class="mt-8 sm:mt-10 text-center">
              <p class="text-responsive-md text-gray-600 mb-16">
                No Docker, no Node.js, no complex configuration. Just a single binary that works.
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
