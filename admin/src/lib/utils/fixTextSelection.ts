// Fix text selection in inputs
export function enableTextSelection() {
	// Remove any CSS that might be preventing text selection
	const style = document.createElement('style');
	style.innerHTML = `
		input, textarea, select, [contenteditable] {
			-webkit-user-select: text !important;
			-moz-user-select: text !important;
			-ms-user-select: text !important;
			user-select: text !important;
			-webkit-user-drag: none !important;
			user-drag: none !important;
		}
		
		/* Ensure cursor is text cursor */
		input:not([type="checkbox"]):not([type="radio"]),
		textarea {
			cursor: text !important;
		}
	`;
	document.head.appendChild(style);
	
	// Remove any event handlers that might be preventing selection
	const inputs = document.querySelectorAll('input, textarea');
	inputs.forEach((input) => {
		// Remove any existing selectstart handlers
		input.onselectstart = null;
		
		// Ensure default behavior is not prevented
		const events = ['mousedown', 'selectstart', 'select'];
		events.forEach(eventName => {
			input.addEventListener(eventName, (e) => {
				e.stopPropagation();
			}, true);
		});
	});
}

// Apply fix when new inputs are added to DOM
export function observeAndFixInputs() {
	enableTextSelection();
	
	// Watch for new inputs being added
	const observer = new MutationObserver(() => {
		enableTextSelection();
	});
	
	observer.observe(document.body, {
		childList: true,
		subtree: true
	});
	
	return observer;
}