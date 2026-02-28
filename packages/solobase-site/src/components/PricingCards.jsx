import { plans, enterpriseFeatures, faqs } from '../data/pricing';

function CheckIcon({ className }) {
  return (
    <svg class={className || 'w-5 h-5 text-green-500 mr-2 mt-0.5 flex-shrink-0'} fill="currentColor" viewBox="0 0 20 20">
      <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
    </svg>
  );
}

function WarningIcon() {
  return (
    <svg class="w-5 h-5 text-yellow-500 mr-2 mt-0.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
      <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clip-rule="evenodd" />
    </svg>
  );
}

function PlanCard({ plan }) {
  const isPopular = plan.popular;

  return (
    <div
      class={`bg-white rounded-lg shadow-lg p-8 border-2 transition-all duration-300 hover:shadow-xl relative ${
        isPopular ? 'border-cyan-500 transform lg:scale-105' : 'border-gray-200 hover:border-cyan-500'
      }`}
    >
      {isPopular && (
        <div class="absolute top-0 right-0 bg-cyan-500 text-white px-4 py-1 text-xs font-bold uppercase rounded-bl-lg rounded-tr-lg">
          Popular
        </div>
      )}
      <div class="text-sm font-semibold text-cyan-600 uppercase tracking-wide mb-2">{plan.name}</div>
      <div class="flex items-baseline mb-4">
        <span class="text-5xl font-bold text-gray-900">${plan.price}</span>
        <span class="text-gray-500 ml-2">/month</span>
      </div>
      <p class="text-gray-600 mb-6">{plan.description}</p>

      <ul class="space-y-4 mb-8">
        {plan.features.map((feat) => (
          <li key={feat.text} class="flex items-start">
            {feat.warning ? <WarningIcon /> : <CheckIcon />}
            <span class={`text-gray-700 ${feat.bold ? 'font-semibold' : ''}`}>{feat.text}</span>
          </li>
        ))}
      </ul>

      <a
        href={plan.ctaUrl}
        class={`block w-full text-center text-white py-3 rounded-lg font-semibold transition-colors ${
          isPopular ? 'bg-cyan-600 hover:bg-cyan-700' : 'bg-gray-900 hover:bg-gray-800'
        }`}
      >
        {plan.cta}
      </a>
    </div>
  );
}

export default function PricingCards() {
  return (
    <div class="min-h-screen bg-gradient-to-b from-gray-50 to-white">
      {/* Header */}
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16 text-center">
        <h1 class="text-5xl font-bold text-gray-900 mb-4">Simple, Transparent Pricing</h1>
        <p class="text-xl text-gray-600 max-w-2xl mx-auto">
          Start with our free tier and scale as you grow. No hidden fees, no surprises.
        </p>
      </div>

      {/* Cards */}
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
          {plans.map((plan) => (
            <PlanCard key={plan.name} plan={plan} />
          ))}
        </div>

        {/* Enterprise */}
        <div class="mt-16 bg-gradient-to-r from-cyan-500 to-blue-600 rounded-lg shadow-xl p-12 text-center text-white">
          <h2 class="text-3xl font-bold mb-4">Enterprise</h2>
          <p class="text-xl mb-6 opacity-90">
            Need more? Custom solutions for large organizations with dedicated infrastructure and white-glove support.
          </p>
          <div class="flex flex-wrap justify-center gap-8 mb-8">
            {enterpriseFeatures.map((feat) => (
              <div key={feat} class="flex items-center">
                <CheckIcon className="w-6 h-6 mr-2" />
                <span>{feat}</span>
              </div>
            ))}
          </div>
          <a
            href="mailto:enterprise@solobase.dev"
            class="inline-block bg-white text-cyan-600 px-8 py-3 rounded-lg font-semibold hover:bg-gray-100 transition-colors"
          >
            Contact Sales
          </a>
        </div>

        {/* FAQ */}
        <div class="mt-24">
          <h2 class="text-3xl font-bold text-center text-gray-900 mb-12">Frequently Asked Questions</h2>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-8 max-w-5xl mx-auto">
            {faqs.map((faq) => (
              <div key={faq.question}>
                <h3 class="text-lg font-semibold text-gray-900 mb-2">{faq.question}</h3>
                <p class="text-gray-600">{faq.answer}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
