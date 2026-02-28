import { render } from 'preact';
import { html } from '@solobase/ui';
import { LogoutPage } from './LogoutPage';
import '../../app.css';

render(html`<${LogoutPage} />`, document.getElementById('app')!);
