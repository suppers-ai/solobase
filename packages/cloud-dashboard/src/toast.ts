/** Simple toast system — uses callback for re-render */

interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

let nextId = 0;
let _toasts: Toast[] = [];
let _listener: (() => void) | null = null;

export function getToasts(): Toast[] {
  return _toasts;
}
export function onToastsChange(fn: () => void) {
  _listener = fn;
}

function add(message: string, type: Toast["type"]) {
  const id = nextId++;
  _toasts = [..._toasts, { id, message, type }];
  _listener?.();
  setTimeout(() => {
    _toasts = _toasts.filter((t) => t.id !== id);
    _listener?.();
  }, 4000);
}

export const toasts = {
  success: (msg: string) => add(msg, "success"),
  error: (msg: string) => add(msg, "error"),
  info: (msg: string) => add(msg, "info"),
};
