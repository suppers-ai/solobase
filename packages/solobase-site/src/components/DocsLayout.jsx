import { useEffect, useRef } from 'preact/hooks';
import DocsSidebar from './DocsSidebar';

export default function DocsLayout({ currentPath, title, description, contentHtml, headings }) {
  const contentRef = useRef(null);

  // Add copy buttons to code blocks after render
  useEffect(() => {
    if (!contentRef.current) return;
    const blocks = contentRef.current.querySelectorAll('pre');
    blocks.forEach((block) => {
      if (block.querySelector('.code-copy-btn')) return;
      const codeEl = block.querySelector('code');
      if (!codeEl) return;

      const btn = document.createElement('button');
      btn.className = 'code-copy-btn';
      btn.textContent = 'copy';
      btn.addEventListener('click', () => {
        navigator.clipboard.writeText(codeEl.textContent).then(() => {
          btn.textContent = 'copied!';
          btn.classList.add('copied');
          setTimeout(() => {
            btn.textContent = 'copy';
            btn.classList.remove('copied');
          }, 2000);
        });
      });
      block.style.position = 'relative';
      block.appendChild(btn);
    });
  }, [contentHtml]);

  // Update active TOC link on scroll
  useEffect(() => {
    function onScroll() {
      const tocLinks = document.querySelectorAll('.docs-toc a');
      const allHeadings = document.querySelectorAll('.docs-content h2, .docs-content h3, .docs-content h4');
      let current = '';
      allHeadings.forEach((h) => {
        if (h.getBoundingClientRect().top <= 100) current = h.id;
      });
      tocLinks.forEach((link) => {
        link.classList.toggle('active', link.getAttribute('href') === '#' + current);
      });
    }
    window.addEventListener('scroll', onScroll);
    return () => window.removeEventListener('scroll', onScroll);
  }, [contentHtml]);

  return (
    <div class="flex min-h-screen">
      <DocsSidebar currentPath={currentPath} />
      <div class="flex-1 flex">
        {/* Content */}
        <main class="flex-1 px-6 py-8 max-w-4xl">
          <article class="docs-content" ref={contentRef}>
            <header class="mb-8">
              <h1 class="text-4xl font-bold text-gray-900 mb-4">{title}</h1>
              {description && <p class="text-xl text-gray-600 mb-4">{description}</p>}
            </header>
            <div class="prose prose-lg max-w-none" dangerouslySetInnerHTML={{ __html: contentHtml }} />
          </article>
        </main>

        {/* Table of Contents */}
        {headings && headings.length > 0 && (
          <aside class="hidden xl:block w-64 pl-8">
            <div class="sticky top-8 flex flex-col" style={{ maxHeight: 'calc(100vh - 4rem)' }}>
              <div class="flex-1">
                <h4 class="text-sm font-semibold text-gray-900 mb-4">On this page</h4>
                <nav class="docs-toc">
                  {headings.map((h) => (
                    <a
                      key={h.id}
                      href={`#${h.id}`}
                      class="text-sm"
                      style={{ paddingLeft: `${(h.level - 2) * 0.75 + 1}rem` }}
                    >
                      {h.text}
                    </a>
                  ))}
                </nav>
              </div>
              <div class="mt-auto pt-4">
                <button
                  onClick={() => window.scrollTo({ top: 0, behavior: 'smooth' })}
                  class="inline-flex items-center text-sm text-gray-600 hover:text-gray-900"
                >
                  <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 11l5-5m0 0l5 5m-5-5v12" />
                  </svg>
                  Back to top
                </button>
              </div>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}
