import { render } from 'preact';
import { html, BlockShell } from '@solobase/ui';
import { MonitoringPage } from './MonitoringPage';
import '@app/app.css';
import './monitoring.css';

render(html`<${BlockShell} title="Dashboard"><${MonitoringPage} /></${BlockShell}>`, document.getElementById('app')!);
