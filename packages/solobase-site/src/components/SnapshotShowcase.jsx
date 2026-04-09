export default function SnapshotShowcase({ onOpenDemo }) {
  return (
    <section style={{ background: 'linear-gradient(180deg, #ffffff 0%, #f9fafb 100%)' }}>
      <div class="container max-w-6xl mx-auto px-6">
        <div class="max-w-5xl mx-auto">
          {/* Browser-like frame */}
          <div class="bg-white rounded-xl shadow-2xl overflow-hidden" style={{ border: '1px solid #e5e7eb' }}>
            {/* Browser header bar */}
            <div class="bg-gray-100 px-4 py-3 border-b border-gray-200 flex items-center">
              <div class="flex space-x-2">
                <div class="w-3 h-3 bg-red-400 rounded-full" />
                <div class="w-3 h-3 bg-yellow-400 rounded-full" />
                <div class="w-3 h-3 bg-green-400 rounded-full" />
              </div>
              <div class="flex-1 text-center">
                <div class="inline-flex items-center bg-white rounded-md px-3 py-1 text-sm text-gray-600">
                  <svg class="w-4 h-4 mr-1 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                  </svg>
                  solobase-demo.fly.dev
                </div>
              </div>
            </div>

            {/* Snapshot Image */}
            <div class="relative">
              <img src="/images/snapshot.png" alt="Solobase Dashboard Screenshot" class="w-full h-auto" style={{ display: 'block' }} />
              <div class="absolute inset-0 flex items-center justify-center">
                <div class="flex flex-col sm:flex-row gap-4 p-6 rounded-lg" style={{ background: 'transparent' }}>
                  <button
                    onClick={onOpenDemo}
                    class="inline-flex items-center justify-center px-8 py-4 rounded-lg font-bold text-white transition-all duration-200 hover:shadow-lg wobble-hover"
                    style={{ background: '#2563eb', minWidth: '220px', fontSize: '1.1rem', boxShadow: '0 2px 10px rgba(37, 99, 235, 0.2)' }}
                  >
                    <svg class="w-6 h-6 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    Try Live Demo
                  </button>
                  <a
                    href="/docs/"
                    class="inline-flex items-center justify-center px-6 py-3 rounded-lg font-medium transition-all duration-200 hover:shadow-lg transform hover:-translate-y-0.5"
                    style={{ background: 'white', border: '2px solid #1f2937', color: '#1f2937', minWidth: '180px' }}
                  >
                    <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
                    </svg>
                    Read the documentation
                  </a>
                </div>
              </div>
            </div>
          </div>

          {/* Features checkmarks */}
          <div class="mt-8 flex flex-wrap justify-center gap-6 text-sm">
            {['SQLite or Postgres', 'JWT Authentication', 'File Storage', 'Stripe Payments', 'Admin Panel'].map((text) => (
              <div key={text} class="flex items-center" style={{ color: '#6b7280' }}>
                <svg class="w-5 h-5 mr-2 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
                {text}
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
