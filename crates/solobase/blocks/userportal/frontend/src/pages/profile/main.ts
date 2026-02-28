import { render } from 'preact';
import { html } from '@solobase/ui';
import { ProfilePage } from './ProfilePage';
import '../../app.css';

render(html`<${ProfilePage} />`, document.getElementById('app')!);
