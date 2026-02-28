import { signal, computed } from '@preact/signals';

export interface Toast {
	id: string;
	type: 'success' | 'error' | 'warning' | 'info';
	message: string;
	title?: string;
	duration?: number;
	dismissible?: boolean;
}

const DEFAULT_DURATION = 5000;

const toastList = signal<Toast[]>([]);
let toastId = 0;

function add(toast: Omit<Toast, 'id'>): string {
	const id = `toast-${++toastId}`;
	const newToast: Toast = {
		id,
		dismissible: true,
		duration: DEFAULT_DURATION,
		...toast
	};

	toastList.value = [...toastList.value, newToast];

	if (newToast.duration && newToast.duration > 0) {
		setTimeout(() => {
			dismiss(id);
		}, newToast.duration);
	}

	return id;
}

function dismiss(id: string) {
	toastList.value = toastList.value.filter(t => t.id !== id);
}

function clear() {
	toastList.value = [];
}

export const toasts = {
	list: toastList,
	count: computed(() => toastList.value.length),
	success: (message: string, title?: string) => add({ type: 'success', message, title }),
	error: (message: string, title?: string) => add({ type: 'error', message, title, duration: 10000 }),
	warning: (message: string, title?: string) => add({ type: 'warning', message, title }),
	info: (message: string, title?: string) => add({ type: 'info', message, title }),
	add,
	dismiss,
	clear
};
