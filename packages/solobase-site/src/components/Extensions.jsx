export default function Extensions() {
  return (
    <section class="py-responsive bg-white">
      <div class="container-responsive">
        <div class="text-center mb-12 sm:mb-16 lg:mb-20">
          <h2 class="text-responsive-lg font-bold text-gray-900 mb-4 sm:mb-6">Extend Without Limits</h2>
          <p class="text-responsive-md text-gray-600 max-w-4xl mx-auto leading-relaxed">
            Official and community extensions further extend Solobase's capabilities. Build exactly what you need with no constraints.
          </p>
        </div>

        <div class="space-y-6 lg:space-y-8 max-w-5xl mx-auto">
          {/* Formula Based Pricing Card */}
          <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <div class="flex flex-col md:flex-row gap-6 p-6 items-center">
              <div class="flex-1 order-2 md:order-1">
                <h3 class="text-xl font-bold text-gray-900 mb-3">Formula Based Pricing</h3>
                <p class="text-gray-600 mb-4 text-sm leading-relaxed">
                  Create pricing for anything with unlimited flexibility. Define variables and formulas, then calculate prices based on user input. The core principle of this extension is:
                </p>
                <div class="bg-gray-900 rounded-lg p-3 overflow-x-auto">
                  <pre class="text-xs">
                    <code class="language-javascript">
                      <span class="text-gray-500">{'// Define your pricing formula'}</span>{'\n'}
                      <span class="text-blue-400">const</span> <span class="text-white">formula</span> = <span class="text-green-400">"base_price * number_of_items * (1 + sales_tax/100)"</span>;{'\n'}
                      {'\n'}
                      <span class="text-gray-500">{'// Set variables'}</span>{'\n'}
                      <span class="text-blue-400">const</span> <span class="text-white">variables</span> = {'{'}{'\n'}
                      {'  '}<span class="text-cyan-400">base_price</span>: <span class="text-orange-400">100</span>,{'\n'}
                      {'  '}<span class="text-cyan-400">number_of_items</span>: <span class="text-orange-400">5</span>,{'\n'}
                      {'  '}<span class="text-cyan-400">sales_tax</span>: <span class="text-orange-400">10</span>{'\n'}
                      {'};'}{'\n'}
                      {'\n'}
                      <span class="text-gray-500">{'// Calculate result'}</span>{'\n'}
                      <span class="text-blue-400">const</span> <span class="text-white">result</span> = <span class="text-yellow-400">calculatePrice</span>(formula, variables);{'\n'}
                      <span class="text-gray-500">{'// Result: 550.00'}</span>
                    </code>
                  </pre>
                </div>
              </div>
              <div class="order-1 md:order-2 flex justify-center items-center">
                <img src="/images/formulapricing.png" alt="Formula Based Pricing Extension" class="rounded-lg" style={{ width: '350px', height: '350px', objectFit: 'contain' }} />
              </div>
            </div>
          </div>

          {/* Dynamic Products Card */}
          <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <div class="flex flex-col md:flex-row gap-6 p-6 items-start">
              <div class="order-1 flex justify-center items-center">
                <img src="/images/dynamicproducts.png" alt="Dynamic Products Extension" class="rounded-lg" style={{ width: '350px', height: '350px', objectFit: 'contain' }} />
              </div>
              <div class="flex-1 order-2 pt-8">
                <h3 class="text-xl font-bold text-gray-900 mb-3">Dynamic Products</h3>
                <p class="text-gray-600 mb-4 text-sm leading-relaxed">
                  Define exactly what products you want to sell and the inputs users need to provide. Create custom product schemas for efficient searching.
                </p>
                <div class="flex flex-wrap gap-2">
                  <span class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800">Custom Fields</span>
                  <span class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800">Smart Search</span>
                  <span class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-800">Flexible Schemas</span>
                </div>
              </div>
            </div>
          </div>

          {/* Cloud Storage Card */}
          <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <div class="flex flex-col md:flex-row gap-6 p-6 items-start">
              <div class="flex-1 order-2 md:order-1 pt-8">
                <h3 class="text-xl font-bold text-gray-900 mb-3">Cloud Storage</h3>
                <p class="text-gray-600 mb-4 text-sm leading-relaxed">
                  Extend storage capabilities with advanced features. Track usage, set limits, and implement extensible sharing.
                </p>
                <ul class="space-y-2">
                  {['Usage tracking & analytics', 'Capacity & bandwidth limits', 'Extensible sharing'].map((text) => (
                    <li key={text} class="flex items-start">
                      <svg class="w-4 h-4 text-green-500 mt-0.5 mr-2 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                      </svg>
                      <span class="text-gray-700 text-sm">{text}</span>
                    </li>
                  ))}
                </ul>
              </div>
              <div class="order-1 md:order-2 flex justify-center items-center">
                <img src="/images/cloudstorage.png" alt="Cloud Storage Extension" class="rounded-lg" style={{ width: '350px', height: '350px', objectFit: 'contain' }} />
              </div>
            </div>
          </div>
        </div>

        <div class="mt-12 sm:mt-16 text-center">
          <p class="text-responsive-sm text-gray-600 mb-6">
            Build your own extensions or explore community contributions
          </p>
          <a
            href="/docs/extensions/"
            class="inline-flex items-center justify-center px-6 py-3 text-white rounded-lg font-medium transition-all duration-200 hover:shadow-lg transform hover:-translate-y-0.5"
            style={{ background: '#1f2937' }}
          >
            Explore Extensions Documentation
            <svg class="ml-2 w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          </a>
        </div>
      </div>
    </section>
  );
}
