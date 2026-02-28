import { h, Fragment } from 'preact';
import htm from 'htm';

// htm maps <> to h("", ...) which crashes Preact. Intercept to use Fragment.
function hWithFragment(type: any, props: any, ...children: any[]) {
	if (type === '') return h(Fragment, props, ...children);
	return h(type, props, ...children);
}

export const html = htm.bind(hWithFragment);
