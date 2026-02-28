import { render } from 'preact';
import { html } from '@solobase/ui';
import { ProductsPage } from './ProductsPage';
import '../../app.css';

render(html`<${ProductsPage} />`, document.getElementById('app')!);
