import { useEffect, useRef } from 'preact/hooks';
import type { RefObject } from 'preact';

export function useClickOutside<T extends HTMLElement>(
	callback: () => void
): RefObject<T> {
	const ref = useRef<T>(null);

	useEffect(() => {
		function handleClick(event: MouseEvent) {
			if (ref.current && !ref.current.contains(event.target as Node) && !event.defaultPrevented) {
				callback();
			}
		}

		document.addEventListener('click', handleClick, true);
		return () => document.removeEventListener('click', handleClick, true);
	}, [callback]);

	return ref;
}
