import { render } from 'preact';
import { html } from '@solobase/ui';
import { SuccessPage } from './SuccessPage';
import '../../app.css';

render(html`<${SuccessPage} />`, document.getElementById('app')!);
