import { render } from 'preact';
import '../css/main.css';
import Header from '../components/Header';
import HackathonBanner from '../components/HackathonBanner';
import PricingCards from '../components/PricingCards';
import Footer from '../components/Footer';

function PricingPage() {
  return (
    <>
      <Header />
      <HackathonBanner />
      <main>
        <PricingCards />
      </main>
      <Footer />
    </>
  );
}

render(<PricingPage />, document.getElementById('app'));
