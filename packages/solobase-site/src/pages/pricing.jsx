import { render } from 'preact';
import '../css/main.css';
import Header from '../components/Header';
import PricingCards from '../components/PricingCards';
import Footer from '../components/Footer';

function PricingPage() {
  return (
    <>
      <Header />
      <main>
        <PricingCards />
      </main>
      <Footer />
    </>
  );
}

render(<PricingPage />, document.getElementById('app'));
