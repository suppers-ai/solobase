import { render } from 'preact';
import { html, BlockShell } from '@solobase/ui';
import { IAMPage } from './IAMPage';
import '@app/app.css';

render(html`<${BlockShell} title="IAM"><${IAMPage} /></${BlockShell}>`, document.getElementById('app')!);
