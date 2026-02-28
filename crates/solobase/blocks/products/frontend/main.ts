import { render } from 'preact';
import { html, BlockShell } from '@solobase/ui';
import { ProductsPage } from './ProductsPage';
import '@app/app.css';
import './products.css';

render(html`<${BlockShell} title="Products"><${ProductsPage} /></${BlockShell}>`, document.getElementById('app')!);
