// Plans are now fetched dynamically from the products API.
// See PricingCards.jsx — it calls GET /api/b/products/products?status=published
// and falls back to hardcoded plans if the API is unavailable.

export const addons = [
  {
    name: 'API Requests',
    unit: '100K',
    price: 1,
  },
  {
    name: 'File Storage',
    unit: '1 GB',
    price: 1,
  },
  {
    name: 'Database Storage',
    unit: '250 MB',
    price: 1,
  },
];

export const enterpriseFeatures = [
  'Unlimited everything',
  'Dedicated infrastructure',
  'Custom SLA',
  'On-premise option',
  'Priority support',
];

export const faqs = [
  {
    question: 'What counts as an API request?',
    answer: 'Every HTTP request to your project\'s API endpoints counts as one request. Static file serving (HTML, CSS, JS, images) does not count.',
  },
  {
    question: 'Can I change plans later?',
    answer: 'Yes! Upgrade or downgrade anytime. Changes take effect immediately with prorated billing.',
  },
  {
    question: 'What happens when I hit a limit?',
    answer: 'We\'ll notify you at 80% usage. If you exceed your limit, API requests return a 429 status. You can add more capacity instantly with add-ons.',
  },
  {
    question: 'What is a project?',
    answer: 'A project is a separate Solobase instance with its own database, storage, users, and subdomain. Each project is fully isolated.',
  },
  {
    question: 'Do you offer refunds?',
    answer: 'Yes, we offer a 14-day money-back guarantee on all plans.',
  },
];
