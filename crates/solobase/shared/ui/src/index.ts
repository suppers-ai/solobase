// HTM helper
export { html } from './htm';

// Layout components
export { Sidebar } from './components/layout/Sidebar';
export { BlockShell } from './components/layout/BlockShell';
export { FeatureShell } from './components/layout/FeatureShell';

// UI components
export { Button } from './components/ui/Button';
export { Modal } from './components/ui/Modal';
export { ConfirmDialog } from './components/ui/ConfirmDialog';
export { ToastContainer } from './components/ui/Toast';
export { LoadingSpinner } from './components/ui/LoadingSpinner';
export { PageHeader } from './components/ui/PageHeader';
export { EmptyState } from './components/ui/EmptyState';
export { StatCard } from './components/ui/StatCard';
export { StatusBadge } from './components/ui/StatusBadge';
export { Pagination } from './components/ui/Pagination';
export { TabNavigation } from './components/ui/TabNavigation';
export { Toggle } from './components/ui/Toggle';
export { Section } from './components/ui/Section';
export { SearchInput } from './components/ui/SearchInput';
export { DataTable } from './components/ui/DataTable';
export { ExportButton } from './components/ui/ExportButton';
export { FilterBar } from './components/ui/FilterBar';

// Stores
export {
	authState, isAuthenticated, currentUser, userRoles, authLoading,
	login, logout, checkAuth, setUser
} from './stores/auth';
export { toasts } from './stores/toast';
export type { Toast } from './stores/toast';

// API client
export { api, authFetch, ErrorHandler } from './api';

// Hooks
export { useClickOutside } from './hooks/useClickOutside';
export { useKeydown } from './hooks/useKeydown';

// Utilities
export { formatPrice, isValidRedirectUrl } from './utils/helpers';
