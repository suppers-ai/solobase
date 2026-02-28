import { useEffect } from 'preact/hooks';
import { siteConfig } from '../data/navigation';

export default function DemoModal({ open, onClose }) {
  useEffect(() => {
    function onKey(e) {
      if (e.key === 'Escape') onClose();
    }
    if (open) {
      document.body.style.overflow = 'hidden';
      document.addEventListener('keydown', onKey);
    }
    return () => {
      document.body.style.overflow = 'auto';
      document.removeEventListener('keydown', onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div class="fixed inset-0 z-50 overflow-y-auto" role="dialog" aria-modal="true">
      <div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0">
        <div class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity" onClick={onClose} />
        <span class="hidden sm:inline-block sm:align-middle sm:h-screen">&#8203;</span>
        <div class="inline-block align-bottom bg-white rounded-xl text-left overflow-hidden shadow-2xl transform transition-all sm:my-8 sm:align-middle sm:max-w-md sm:w-full">
          <div class="bg-white px-6 pt-6 pb-5">
            <div class="text-center mb-5">
              <div class="flex items-center justify-center mb-3">
                <div class="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100">
                  <svg class="h-6 w-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </div>
              </div>
              <h3 class="text-xl font-semibold text-gray-900">Welcome to the Solobase Demo</h3>
            </div>
            <div class="space-y-4">
              <p class="text-sm text-gray-600">This is a demo instance of Solobase. Please note:</p>
              <ul class="text-sm text-gray-600 space-y-2 pl-5">
                <li class="flex items-start">
                  <span class="text-gray-400 mr-2">&bull;</span>
                  <span>The demo is in <strong class="text-gray-800">read-only mode</strong></span>
                </li>
                <li class="flex items-start">
                  <span class="text-gray-400 mr-2">&bull;</span>
                  <span>Data is <strong class="text-gray-800">reset regularly</strong></span>
                </li>
                <li class="flex items-start">
                  <span class="text-gray-400 mr-2">&bull;</span>
                  <span>Some features may be limited</span>
                </li>
              </ul>
              <div class="bg-gray-50 rounded-lg p-4 border border-gray-200">
                <p class="text-sm font-semibold text-gray-900 mb-3">Login Credentials:</p>
                <div class="space-y-3">
                  <div class="grid grid-cols-3 gap-2 items-center">
                    <span class="text-sm text-gray-600">Email:</span>
                    <code class="col-span-2 text-sm bg-white px-3 py-1.5 rounded border border-gray-300 text-center" style={{ fontFamily: "'Courier New', monospace" }}>
                      admin@example.com
                    </code>
                  </div>
                  <div class="grid grid-cols-3 gap-2 items-center">
                    <span class="text-sm text-gray-600">Password:</span>
                    <code class="col-span-2 text-sm bg-white px-3 py-1.5 rounded border border-gray-300 text-center" style={{ fontFamily: "'Courier New', monospace" }}>
                      admin123
                    </code>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div class="bg-gray-50 px-6 py-4 flex flex-row-reverse gap-3">
            <a
              href={siteConfig.demoUrl}
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex justify-center items-center rounded-lg px-5 py-2.5 bg-blue-600 text-sm font-medium text-white hover:bg-blue-700 transition-colors"
            >
              Continue to Demo
            </a>
            <button
              type="button"
              onClick={onClose}
              class="inline-flex justify-center items-center rounded-lg px-5 py-2.5 bg-white border border-gray-300 text-sm font-medium text-gray-700 hover:bg-gray-50 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
