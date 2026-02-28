import { render } from 'preact';
import { html, BlockShell } from '@solobase/ui';
import { LogsPage } from './LogsPage';
import '@app/app.css';

render(html`<${BlockShell} title="Logs"><${LogsPage} /></${BlockShell}>`, document.getElementById('app')!);
