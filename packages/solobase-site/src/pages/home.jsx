import { render } from 'preact';
import { useState } from 'preact/hooks';
import '../css/main.css';
import Header from '../components/Header';
import Hero from '../components/Hero';
import Footer from '../components/Footer';
import DemoModal from '../components/DemoModal';

const platforms = [
  { name: 'Linux', url: 'https://solobase.dev/download/linux', extract: 'tar xz', run: './solobase' },
  { name: 'Linux ARM', url: 'https://solobase.dev/download/linux-arm', extract: 'tar xz', run: './solobase' },
  { name: 'macOS', url: 'https://solobase.dev/download/mac', extract: 'tar xz', run: './solobase' },
  { name: 'macOS Intel', url: 'https://solobase.dev/download/mac-intel', extract: 'tar xz', run: './solobase' },
  { name: 'Windows', url: 'https://solobase.dev/download/windows', extract: null, run: 'solobase.exe' },
];

function CodeBlock() {
  const [active, setActive] = useState(0);
  const p = platforms[active];
  const comment = { color: '#6a9955' };
  const cmd = { color: '#d4d4d4' };

  return (
    <div style={{ borderRadius: '0.5rem', overflow: 'hidden', background: '#1e1e1e' }}>
      <div style={{ display: 'flex', gap: 0, borderBottom: '1px solid #2d2d30', overflowX: 'auto' }}>
        {platforms.map((pl, i) => (
          <button
            key={pl.name}
            onClick={() => setActive(i)}
            style={{
              padding: '0.5rem 1rem',
              background: i === active ? '#1e1e1e' : '#252526',
              color: i === active ? '#d4d4d4' : '#6b7280',
              border: 'none',
              borderBottom: i === active ? '2px solid #06b6d4' : '2px solid transparent',
              cursor: 'pointer',
              fontSize: '0.75rem',
              fontFamily: "'Consolas', 'Monaco', 'Courier New', monospace",
              whiteSpace: 'nowrap',
              transition: 'color 0.15s',
            }}
          >
            {pl.name}
          </button>
        ))}
      </div>
      <pre style={{
        margin: 0,
        padding: '1.25rem 1.5rem',
        fontSize: '0.8rem',
        lineHeight: 1.8,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-all',
        color: '#d4d4d4',
        fontFamily: "'Consolas', 'Monaco', 'Courier New', monospace",
      }}>
        <span style={comment}># Download the latest release</span>{'\n'}
        {p.extract ? (
          <>
            <span style={cmd}>{`curl -sSL ${p.url} | ${p.extract}`}</span>{'\n'}
          </>
        ) : (
          <>
            <span style={cmd}>{`curl -sSLO ${p.url}`}</span>{'\n'}
            <span style={cmd}>{`tar -xf solobase-windows-amd64.zip`}</span>{'\n'}
          </>
        )}
        {'\n'}
        <span style={comment}># Run it</span>{'\n'}
        <span style={cmd}>{p.run}</span>{'\n\n'}
        <span style={comment}>{"# That's it. Auth, database, storage, products, admin panel — all running on :8090"}</span>
      </pre>
    </div>
  );
}

function GetStarted({ onOpenDemo }) {
  return (
    <section style={{ background: '#ffffff', padding: '0 1.5rem 4rem' }}>
      <div class="max-w-2xl mx-auto">
        <CodeBlock />
        <div class="mt-6 flex justify-center gap-4 flex-wrap">
          <a
            href="#"
            onClick={(e) => { e.preventDefault(); onOpenDemo?.(); }}
            style={{
              display: 'inline-block',
              padding: '0.75rem 2rem',
              background: '#1f2937',
              color: 'white',
              borderRadius: '0.5rem',
              fontWeight: 600,
              transition: 'background 0.2s',
              cursor: 'pointer',
            }}
            onMouseOver={(e) => (e.currentTarget.style.background = '#374151')}
            onMouseOut={(e) => (e.currentTarget.style.background = '#1f2937')}
          >
            Demo
          </a>
          <a
            href="/docs/"
            style={{
              display: 'inline-block',
              padding: '0.75rem 2rem',
              background: 'white',
              color: '#1f2937',
              border: '2px solid #e5e7eb',
              borderRadius: '0.5rem',
              fontWeight: 600,
              transition: 'all 0.2s',
            }}
            onMouseOver={(e) => { e.currentTarget.style.borderColor = '#1f2937'; }}
            onMouseOut={(e) => { e.currentTarget.style.borderColor = '#e5e7eb'; }}
          >
            Read the documentation
          </a>
        </div>
      </div>
    </section>
  );
}

function HomePage() {
  const [demoOpen, setDemoOpen] = useState(false);

  return (
    <>
      <Header onOpenDemo={() => setDemoOpen(true)} />
      <main>
        <Hero />
        <GetStarted onOpenDemo={() => setDemoOpen(true)} />
      </main>
      <Footer />
      <DemoModal open={demoOpen} onClose={() => setDemoOpen(false)} />
    </>
  );
}

render(<HomePage />, document.getElementById('app'));
