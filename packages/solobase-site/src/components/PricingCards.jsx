import { useState, useEffect } from 'preact/hooks';
import { faqs, addons } from '../data/pricing';

function CheckIcon({ className }) {
  return (
    <svg class={className || 'w-5 h-5 text-green-500 mr-2 mt-0.5 flex-shrink-0'} fill="currentColor" viewBox="0 0 20 20">
      <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
    </svg>
  );
}

function XIcon() {
  return (
    <svg class="w-5 h-5 text-gray-300 mr-2 mt-0.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
      <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
    </svg>
  );
}

function Spinner() {
  return (
    <div class="flex justify-center py-16">
      <div style={{
        width: '40px', height: '40px', border: '3px solid #e2e8f0',
        borderTopColor: '#fe6627', borderRadius: '50%',
        animation: 'spin 0.6s linear infinite',
      }} />
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

/** Parse product metadata JSON string into features list for display. */
function parseFeatures(product) {
  try {
    const meta = typeof product.metadata === 'string' ? JSON.parse(product.metadata) : (product.metadata || {});
    const features = [];

    if (meta.max_projects) features.push({ text: `${meta.max_projects} projects`, included: true });
    features.push({ text: 'Dedicated database & storage per project', included: true, bold: true });
    if (meta.max_requests) features.push({ text: `${formatNumber(meta.max_requests)} API requests/month`, included: true });
    if (meta.d1_storage) features.push({ text: `${meta.d1_storage} database storage`, included: true });
    if (meta.r2_storage) features.push({ text: `${meta.r2_storage} file storage`, included: true });

    features.push({ text: 'Subdomain', included: true });
    features.push({ text: 'SSL included', included: true });

    if (meta.custom_domain !== undefined) {
      features.push({ text: 'Custom domain', included: meta.custom_domain === 'true' || meta.custom_domain === true });
    }

    if (meta.support) features.push({ text: meta.support, included: true });

    return features;
  } catch {
    return [{ text: product.description || 'See details', included: true }];
  }
}

function formatNumber(n) {
  const num = parseInt(n, 10);
  if (num >= 1000000) return `${num / 1000000}M`;
  if (num >= 1000) return `${num / 1000}K`;
  return String(num);
}

/** Fallback plans if API is unavailable. */
const FALLBACK_PLANS = [
  { name: 'Starter', price: 5, description: 'For side projects and small apps', popular: false,
    features: [
      { text: '2 projects', included: true },
      { text: 'Dedicated database & storage per project', included: true, bold: true },
      { text: '500K API requests/month', included: true },
      { text: '500MB database storage', included: true }, { text: '2GB file storage', included: true },
      { text: 'Subdomain', included: true }, { text: 'SSL included', included: true },
      { text: 'Custom domain', included: false },
      { text: 'Add-ons', included: false },
    ],
  },
  { name: 'Pro', price: 25, description: 'For growing apps and production workloads', popular: true,
    features: [
      { text: '10 projects', included: true },
      { text: 'Dedicated database & storage per project', included: true, bold: true },
      { text: '3M API requests/month', included: true },
      { text: '5GB database storage', included: true }, { text: '20GB file storage', included: true },
      { text: 'Custom domain support', included: true }, { text: 'SSL included', included: true },
      { text: 'Priority email support', included: true },
      { text: 'Add-ons available', included: true },
    ],
  },
];

const planLogos = {
  starter: '/images/starter_logo.png',
  pro: '/images/pro_logo.png',
};

function PlanCard({ plan }) {
  const logo = planLogos[plan.name.toLowerCase()];

  return (
    <div
      class={`bg-white rounded-2xl shadow-lg p-8 border-2 transition-all duration-300 hover:shadow-xl flex flex-col relative ${plan.popular ? 'border-primary-500' : 'border-gray-200'}`}
    >
      {plan.popular && (
        <div class="absolute -top-3 left-1/2 -translate-x-1/2 bg-primary-500 text-white text-xs font-semibold px-3 py-1 rounded-full">
          Coolest Option
        </div>
      )}
      <div class="flex items-center justify-between mb-2">
        <div class="text-sm font-semibold text-primary-600 uppercase tracking-wide">{plan.name}</div>
        {logo && <img src={logo} alt={plan.name} class="h-10 object-contain" />}
      </div>
      <div class="flex items-baseline mb-2">
        <span class="text-5xl font-bold text-gray-900">${plan.price}</span>
        <span class="text-gray-500 ml-2">/month</span>
      </div>
      <p class="text-gray-600 mb-6">{plan.description}</p>

      <ul class="space-y-3 mb-8 flex-1">
        {plan.features.map((feat) => (
          <li key={feat.text} class="flex items-start">
            {feat.included ? <CheckIcon /> : <XIcon />}
            <span class={`${feat.included ? 'text-gray-700' : 'text-gray-400'} ${feat.bold ? 'font-semibold' : ''}`}>
              {feat.text}
            </span>
          </li>
        ))}
      </ul>

      <a
        href={`https://cloud.solobase.dev/blocks/auth/?plan=${plan.slug || plan.name.toLowerCase()}`}
        class="block w-full text-center text-white py-3 rounded-lg font-semibold transition-colors bg-gray-900 hover:bg-gray-800"
      >
        Get Started
      </a>
    </div>
  );
}

function AddonCard({ addon }) {
  return (
    <div class="bg-white rounded-xl border border-gray-200 p-4 hover:border-primary-300 transition-colors flex items-center justify-between">
      <span class="font-semibold text-gray-900 text-sm">{addon.name}</span>
      <span class="text-gray-600 text-sm whitespace-nowrap">
        <span class="font-bold text-gray-900">${addon.price}</span>/{addon.unit}
      </span>
    </div>
  );
}

export default function PricingCards() {
  const [plans, setPlans] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Fetch published products from the API
    fetch('https://cloud.solobase.dev/api/b/products/catalog')
      .then(r => r.ok ? r.json() : Promise.reject())
      .then(data => {
        const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
        if (records.length > 0) {
          // Flatten: API returns { id, data: {...} }
          const flat = records.map(r => ({ id: r.id, ...r.data }));
          // Sort by sort_order, then price
          const sorted = flat.sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0) || (a.price || 0) - (b.price || 0));
          const mapped = sorted.map(p => {
            const meta = typeof p.metadata === 'string' ? JSON.parse(p.metadata || '{}') : (p.metadata || {});
            return {
              name: p.name,
              price: p.price || 0,
              description: p.description || '',
              slug: p.slug || p.name.toLowerCase(),
              popular: meta.popular === true || meta.popular === 'true',
              features: parseFeatures(p),
            };
          });
          setPlans(mapped);
        } else {
          setPlans(FALLBACK_PLANS);
        }
      })
      .catch(() => {
        setPlans(FALLBACK_PLANS);
      })
      .finally(() => setLoading(false));
  }, []);

  return (
    <div class="min-h-screen bg-gradient-to-b from-gray-50 to-white">
      {/* Header */}
      <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16 text-center">
        <h1 class="text-5xl font-bold text-gray-900 mb-4">Simple, Transparent Pricing</h1>
        <p class="text-xl text-gray-600 max-w-2xl mx-auto">
          Two plans, no surprises. Scale with add-ons when you need more.
        </p>
      </div>

      {/* Plan Cards */}
      <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pb-8">
        {loading ? (
          <Spinner />
        ) : (
          <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
            {plans.map((plan) => (
              <PlanCard key={plan.name} plan={plan} />
            ))}
          </div>
        )}
      </div>

      {/* Cloudflare badge */}
      <div class="flex flex-col items-center gap-2 pb-16">
        <span class="text-sm text-gray-500">Deployed on the edge, powered by</span>
        <img src="/images/cloudflare.png" alt="Cloudflare" style={{ height: '28px' }} />
      </div>

      {/* Add-ons */}
      <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pb-16">
        <h2 class="text-2xl font-bold text-center text-gray-900 mb-2">Need More? Add-ons</h2>
        <p class="text-center text-gray-500 mb-8">Available on the Pro plan</p>
        <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-6">
          {addons.map((addon) => (
            <AddonCard key={addon.name} addon={addon} />
          ))}
        </div>
      </div>

      {/* Enterprise */}
      <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pb-16">
        <div class="bg-white border-2 border-primary-400 rounded-xl px-8 py-6 flex flex-col sm:flex-row items-center justify-between gap-4">
          <div>
            <h3 class="text-lg font-bold text-gray-900">Enterprise</h3>
            <p class="text-gray-600 text-sm">Custom solutions for large organizations with dedicated infrastructure.</p>
          </div>
          <a
            href="https://discord.gg/jKqMcbrVzm"
            target="_blank"
            rel="noopener noreferrer"
            class="shrink-0 inline-block bg-primary-600 text-white px-6 py-2 rounded-lg font-semibold hover:bg-primary-700 transition-colors"
          >
            Contact Us
          </a>
        </div>
      </div>

      {/* FAQ */}
      <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
        <h2 class="text-3xl font-bold text-center text-gray-900 mb-12">Frequently Asked Questions</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
          {faqs.map((faq) => (
            <div key={faq.question}>
              <h3 class="text-lg font-semibold text-gray-900 mb-2">{faq.question}</h3>
              <p class="text-gray-600">{faq.answer}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
