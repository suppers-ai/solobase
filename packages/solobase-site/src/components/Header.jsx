import { useState } from 'preact/hooks';
import { mainMenu, siteConfig } from '../data/navigation';

export default function Header({ onOpenDemo }) {
  const [mobileOpen, setMobileOpen] = useState(false);

  function toggleMobile() {
    setMobileOpen((v) => !v);
  }

  return (
    <header style={{ background: 'white', borderBottom: '1px solid #e5e7eb', position: 'sticky', top: 0, zIndex: 40 }}>
      <nav class="container max-w-6xl mx-auto px-6">
        <div class="flex items-center justify-between h-16">
          {/* Logo */}
          <div class="flex items-center">
            <a href="/" class="flex items-center">
              <img src="/images/logo_long.png" alt="Solobase" style={{ height: '40px', width: 'auto' }} />
            </a>
          </div>

          {/* Desktop Navigation */}
          <div class="hidden md:flex items-center space-x-6">
            {mainMenu.map((item) =>
              item.isDemo ? (
                <a
                  key={item.name}
                  href="#"
                  onClick={(e) => { e.preventDefault(); onOpenDemo?.(); }}
                  class="cursor-pointer"
                  style={{ color: '#6b7280', transition: 'color 0.2s' }}
                  onMouseOver={(e) => (e.currentTarget.style.color = '#06b6d4')}
                  onMouseOut={(e) => (e.currentTarget.style.color = '#6b7280')}
                >
                  {item.name}
                </a>
              ) : (
                <a
                  key={item.name}
                  href={item.url}
                  style={{ color: '#6b7280', transition: 'color 0.2s' }}
                  onMouseOver={(e) => (e.currentTarget.style.color = '#06b6d4')}
                  onMouseOut={(e) => (e.currentTarget.style.color = '#6b7280')}
                  {...(item.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                >
                  {item.name}
                </a>
              )
            )}
          </div>

          {/* Mobile menu button */}
          <div class="md:hidden">
            <button type="button" style={{ padding: '0.5rem', color: '#6b7280' }} onClick={toggleMobile}>
              {mobileOpen ? (
                <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              ) : (
                <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
                </svg>
              )}
            </button>
          </div>
        </div>
      </nav>

      {/* Mobile Navigation */}
      {mobileOpen && (
        <div class="md:hidden" style={{ background: 'white', borderTop: '1px solid #e5e7eb' }}>
          <nav class="px-6 py-4">
            {mainMenu.map((item) =>
              item.isDemo ? (
                <a
                  key={item.name}
                  href="#"
                  onClick={(e) => { e.preventDefault(); onOpenDemo?.(); toggleMobile(); }}
                  class="block py-2 cursor-pointer"
                  style={{ color: '#6b7280' }}
                >
                  {item.name}
                </a>
              ) : (
                <a
                  key={item.name}
                  href={item.url}
                  class="block py-2"
                  style={{ color: '#6b7280' }}
                  {...(item.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                >
                  {item.name}
                </a>
              )
            )}
          </nav>
        </div>
      )}
    </header>
  );
}
