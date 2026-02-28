import { render } from 'preact';
import { html } from '@solobase/ui';
import { CheckoutPage } from './CheckoutPage';
import '../../app.css';

render(html`<${CheckoutPage} />`, document.getElementById('app')!);
