import { writable } from 'svelte/store';

export interface Toast {
	id: string;
	type: 'success' | 'error' | 'warning' | 'info';
	message: string;
	title?: string;
	duration?: number;
	dismissible?: boolean;
}

const DEFAULT_DURATION = 5000;

function createToastStore() {
	const { subscribe, update } = writable<Toast[]>([]);

	let toastId = 0;

	function add(toast: Omit<Toast, 'id'>) {
		const id = `toast-${++toastId}`;
		const newToast: Toast = {
			id,
			dismissible: true,
			duration: DEFAULT_DURATION,
			...toast
		};

		update(toasts => [...toasts, newToast]);

		if (newToast.duration && newToast.duration > 0) {
			setTimeout(() => {
				dismiss(id);
			}, newToast.duration);
		}

		return id;
	}

	function dismiss(id: string) {
		update(toasts => toasts.filter(t => t.id !== id));
	}

	function clear() {
		update(() => []);
	}

	return {
		subscribe,
		success: (message: string, title?: string) => add({ type: 'success', message, title }),
		error: (message: string, title?: string) => add({ type: 'error', message, title, duration: 10000 }),
		warning: (message: string, title?: string) => add({ type: 'warning', message, title }),
		info: (message: string, title?: string) => add({ type: 'info', message, title }),
		add,
		dismiss,
		clear
	};
}

export const toasts = createToastStore();