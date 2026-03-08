import { h, render } from 'preact';
import { App } from './App';
import '../../../frontend/src/app.css';

render(h(App, null), document.getElementById('app')!);
