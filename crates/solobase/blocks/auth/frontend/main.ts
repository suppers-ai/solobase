import { render } from 'preact';
import { html } from '@solobase/ui';
import { LoginPage } from './LoginPage';
import '@app/app.css';

render(html`<${LoginPage} />`, document.getElementById('app')!);
