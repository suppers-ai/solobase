import { render } from 'preact';
import { html, BlockShell } from '@solobase/ui';
import { WaferPage } from './WaferPage';
import '@app/app.css';

render(html`<${BlockShell} title="Blocks & Flows"><${WaferPage} /></${BlockShell}>`, document.getElementById('app')!);
