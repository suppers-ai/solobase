import { render } from 'preact';
import { useState } from 'preact/hooks';
import '../css/main.css';
import Header from '../components/Header';
import HackathonBanner from '../components/HackathonBanner';
import Hero from '../components/Hero';
import SnapshotShowcase from '../components/SnapshotShowcase';
import Features from '../components/Features';
import Extensions from '../components/Extensions';
import Footer from '../components/Footer';
import DemoModal from '../components/DemoModal';

function HomePage() {
  const [demoOpen, setDemoOpen] = useState(false);

  return (
    <>
      <Header onOpenDemo={() => setDemoOpen(true)} />
      <HackathonBanner />
      <main>
        <Hero />
        <SnapshotShowcase onOpenDemo={() => setDemoOpen(true)} />
        <Features />
        <Extensions />
      </main>
      <Footer />
      <DemoModal open={demoOpen} onClose={() => setDemoOpen(false)} />
    </>
  );
}

render(<HomePage />, document.getElementById('app'));
