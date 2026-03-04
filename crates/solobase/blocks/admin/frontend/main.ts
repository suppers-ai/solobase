import { render } from 'preact';
import { html, BlockShell } from '@solobase/ui';
import { WafflePage } from './WafflePage';
import '@app/app.css';

render(html`<${BlockShell} title="Blocks & Flows"><${WafflePage} /></${BlockShell}>`, document.getElementById('app')!);
