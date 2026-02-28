import { render } from 'preact';
import { html } from '@solobase/ui';
import { OAuthCallbackPage } from './OAuthCallbackPage';
import '../../app.css';

render(html`<${OAuthCallbackPage} />`, document.getElementById('app')!);
