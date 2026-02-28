import { render } from 'preact';
import { html } from '@solobase/ui';
import { ProfileProductsPage } from './ProfileProductsPage';
import '../../app.css';

render(html`<${ProfileProductsPage} />`, document.getElementById('app')!);
