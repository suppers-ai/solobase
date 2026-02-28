import { useEffect } from 'preact/hooks';

export function useKeydown(
	key: string,
	callback: (event: KeyboardEvent) => void,
	enabled = true
): void {
	useEffect(() => {
		if (!enabled) return;

		function handleKeydown(event: KeyboardEvent) {
			if (event.key === key) {
				callback(event);
			}
		}

		document.addEventListener('keydown', handleKeydown);
		return () => document.removeEventListener('keydown', handleKeydown);
	}, [key, callback, enabled]);
}
