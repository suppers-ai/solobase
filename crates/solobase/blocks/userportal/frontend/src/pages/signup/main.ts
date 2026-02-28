import { render } from 'preact';
import { html } from '@solobase/ui';
import { SignupPage } from './SignupPage';
import '../../app.css';

render(html`<${SignupPage} />`, document.getElementById('app')!);
