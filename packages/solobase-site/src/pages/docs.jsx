import { render } from 'preact';
import { useState, useEffect, useMemo } from 'preact/hooks';
import '../css/main.css';
import Header from '../components/Header';
import HackathonBanner from '../components/HackathonBanner';
import DocsLayout from '../components/DocsLayout';
import Footer from '../components/Footer';
import { parseFrontMatter, renderMarkdown, extractHeadings } from '../lib/markdown';

// Import all markdown docs at build time
const markdownFiles = import.meta.glob('/content/docs/**/*.md', { query: '?raw', import: 'default', eager: true });

// Build a lookup map: path -> { data, content, html, headings }
function buildDocsMap() {
  const docs = {};
  for (const [filePath, raw] of Object.entries(markdownFiles)) {
    const { data, content } = parseFrontMatter(raw);
    // Convert file path to URL path
    // /content/docs/_index.md -> /docs/
    // /content/docs/installation/_index.md -> /docs/installation/
    // /content/docs/extensions/cloud-storage.md -> /docs/extensions/cloud-storage/
    let urlPath = filePath
      .replace('/content', '')
      .replace('/_index.md', '/')
      .replace('.md', '/');

    docs[urlPath] = {
      data,
      content,
      html: renderMarkdown(content),
      headings: extractHeadings(content),
    };
  }
  return docs;
}

function DocsPage() {
  const docsMap = useMemo(() => buildDocsMap(), []);

  function getPath() {
    let path = window.location.pathname;
    if (!path.endsWith('/')) path += '/';
    return path;
  }

  const [currentPath, setCurrentPath] = useState(getPath);

  useEffect(() => {
    function onPopState() {
      setCurrentPath(getPath());
    }
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, []);

  const doc = docsMap[currentPath] || docsMap['/docs/'];
  const title = doc?.data?.title || 'Documentation';
  const description = doc?.data?.description || '';

  return (
    <>
      <Header />
      <HackathonBanner />
      <DocsLayout
        currentPath={currentPath}
        title={title}
        description={description}
        contentHtml={doc?.html || '<p>Page not found.</p>'}
        headings={doc?.headings || []}
      />
      <Footer />
    </>
  );
}

render(<DocsPage />, document.getElementById('app'));
