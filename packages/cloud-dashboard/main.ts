import { h, render } from 'preact';
import { App } from './App';
import '@app/app.css';

render(h(App, null), document.getElementById('app')!);
