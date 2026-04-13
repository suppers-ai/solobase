import { render } from 'preact';
import '../css/main.css';
import Header from '../components/Header';
import Footer from '../components/Footer';
import UseCaseCard from '../components/UseCaseCard';
import { groups, useCases } from '../data/use-cases';

function UseCasesPage() {
  return (
    <>
      <Header />
      <main>
        {/* Hero */}
        <section class="text-center py-16 sm:py-20 px-6 max-w-3xl mx-auto">
          <h1 class="text-responsive-xl font-bold text-gray-800 mb-4">
            Rethink what a{' '}
            <span style={{ color: 'var(--primary)' }}>backend</span> can do
          </h1>
          <p class="text-responsive-sm text-gray-500 max-w-xl mx-auto">
            A backend that runs in a browser tab, ships as a single binary, or
            scales across the globe. Same codebase, limitless possibilities.
          </p>
        </section>

        {/* Group pills */}
        <div class="flex justify-center gap-3 px-6 pb-12 flex-wrap">
          {groups.map((g) => (
            <a
              key={g.id}
              href={`#${g.id}`}
              class="px-5 py-2.5 rounded-full text-sm font-medium transition-all duration-200 hover:opacity-80"
              style={{
                background: g.color.pillBg,
                border: `2px solid ${g.color.pillBorder}`,
                color: g.color.pillBorder,
              }}
            >
              {g.label}
            </a>
          ))}
        </div>

        {/* Sections */}
        {groups.map((g, i) => {
          const items = useCases.filter((uc) => uc.group === g.id);
          return (
            <div key={g.id}>
              {i > 0 && (
                <hr class="border-t border-gray-200 max-w-6xl mx-auto" />
              )}
              <section
                id={g.id}
                class="max-w-6xl mx-auto px-6 py-12 scroll-mt-20"
              >
                <div class="flex items-center gap-3 mb-2">
                  <div
                    class="w-3 h-3 rounded-full"
                    style={{ background: g.color.dot }}
                  />
                  <h2 class="text-2xl sm:text-3xl font-bold text-gray-800">
                    {g.heading}
                  </h2>
                </div>
                <p class="text-gray-500 mb-8">{g.subtitle}</p>
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
                  {items.map((uc) => (
                    <UseCaseCard
                      key={uc.title}
                      title={uc.title}
                      description={uc.description}
                      tagLabel={g.tag}
                      color={g.color}
                    />
                  ))}
                </div>
              </section>
            </div>
          );
        })}

        {/* Bottom CTA */}
        <section class="text-center py-16 bg-gray-50">
          <h2 class="text-responsive-lg font-bold text-gray-800 mb-3">
            Ready to build?
          </h2>
          <p class="text-gray-500 text-lg mb-6">
            Same codebase runs everywhere. Start in the browser, deploy to the
            edge.
          </p>
          <div class="flex justify-center gap-3 flex-wrap">
            <a
              href="https://demo.solobase.dev"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center px-6 py-3 rounded-lg font-semibold text-white transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg"
              style={{ background: 'var(--primary)' }}
              onMouseOver={(e) =>
                (e.currentTarget.style.background = 'var(--primary-hover)')
              }
              onMouseOut={(e) =>
                (e.currentTarget.style.background = 'var(--primary)')
              }
            >
              Try in Browser
            </a>
            <a
              href="/"
              class="inline-flex items-center px-6 py-3 rounded-lg font-semibold text-white bg-gray-800 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:bg-gray-700"
            >
              Download Binary
            </a>
          </div>
        </section>
      </main>
      <Footer />
    </>
  );
}

render(<UseCasesPage />, document.getElementById('app'));
