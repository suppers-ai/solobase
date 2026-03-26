import { html } from '../../htm';
import { useState, useEffect } from 'preact/hooks';
import { useClickOutside } from '../../hooks/useClickOutside';
import { currentUser, userRoles, logout } from '../../stores/auth';
import { API_BASE } from '../../api';
import {
	ChevronsLeft, ChevronsRight, LogOut, Settings,
	LayoutDashboard, Users, Database, HardDrive, Shield,
	FileText, Table, Package, GitBranch, ShoppingBag, ScrollText
} from 'lucide-preact';

const iconMap: Record<string, any> = {
	'layout-dashboard': LayoutDashboard,
	'users': Users,
	'database': Database,
	'hard-drive': HardDrive,
	'shield': Shield,
	'settings': Settings,
	'file-text': FileText,
	'table': Table,
	'package': Package,
	'git-branch': GitBranch,
	'shopping-bag': ShoppingBag,
	'scroll-text': ScrollText,
};

interface NavItem {
	title: string;
	href: string;
	icon: string;
	separator?: boolean;
}

export function Sidebar() {
	const [collapsed, setCollapsed] = useState(false);
	const [navItems, setNavItems] = useState<NavItem[]>([]);
	const [showProfileMenu, setShowProfileMenu] = useState(false);
	const profileRef = useClickOutside<HTMLDivElement>(() => setShowProfileMenu(false));

	const [currentHash, setCurrentHash] = useState(() =>
		typeof window !== 'undefined' ? window.location.hash : ''
	);

	const user = currentUser.value;
	const roles = userRoles.value;
	const currentPath = typeof window !== 'undefined' ? window.location.pathname : '';

	useEffect(() => {
		fetch(`${API_BASE}/nav`, { credentials: 'include' })
			.then(r => r.json())
			.then(items => setNavItems(items))
			.catch(() => {});
	}, []);

	// Check if a nav item should be active
	function isActive(item: NavItem): boolean {
		const [itemPath, itemHash] = item.href.split('#');
		if (itemHash) {
			return currentPath === itemPath && currentHash === '#' + itemHash;
		}
		return currentPath === item.href;
	}

	useEffect(() => {
		function onHashChange() {
			setCurrentHash(window.location.hash);
		}
		window.addEventListener('hashchange', onHashChange);
		return () => window.removeEventListener('hashchange', onHashChange);
	}, []);

	const userInitial = user?.email ? user.email[0].toUpperCase() : '?';

	return html`
		<nav class=${`sidebar${collapsed ? ' collapsed' : ''}`}>
			<div class="sidebar-header">
				<a href="/blocks/admin/frontend/" class="sidebar-logo">
					${!collapsed
						? html`<img src="/logo_long.png" alt="Solobase" class="sidebar-logo-long" />`
						: html`<img src="/logo.png" alt="Solobase" class="sidebar-logo-icon" />`
					}
				</a>
			</div>

			<div class="sidebar-nav">
				${navItems.map(item => {
					const IconComponent = iconMap[item.icon];
					return html`
					<${'' /* fragment */}>
						${item.separator ? html`<div class="nav-separator" />` : null}
						<div class="nav-item" key=${item.href}>
							<a
								href=${item.href}
								class=${`nav-link${isActive(item) ? ' active' : ''}`}
							>
								${IconComponent ? html`<${IconComponent} size=${18} class="nav-icon" />` : null}
								<span class="nav-text">${item.title}</span>
								${collapsed ? html`<span class="nav-tooltip">${item.title}</span>` : null}
							</a>
						</div>
					<//>
				`})}

				<button
					class="sidebar-toggle"
					onClick=${() => setCollapsed(c => !c)}
					type="button"
					aria-label=${collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
				>
					${collapsed
						? html`<${ChevronsRight} size=${14} />`
						: html`<${ChevronsLeft} size=${14} />`
					}
				</button>
			</div>

			<div class="sidebar-user-container" ref=${profileRef}>
				<button
					class="sidebar-user"
					onClick=${() => setShowProfileMenu(v => !v)}
					type="button"
				>
					<div class="user-avatar">${userInitial}</div>
					${!collapsed ? html`
						<div class="user-info">
							<div class="user-email">${user?.email || ''}</div>
						</div>
					` : null}
				</button>

				${showProfileMenu ? html`
					<div class=${`profile-menu${collapsed ? ' profile-menu-collapsed' : ''}`}>
						<div class="profile-menu-header">
							<div class="profile-menu-avatar">${userInitial}</div>
							<div class="profile-menu-info">
								<div class="profile-menu-email">${user?.email || ''}</div>
								<div class="profile-menu-role">${roles.includes('admin') ? 'Admin' : 'User'}</div>
							</div>
						</div>
						<div class="profile-menu-divider" />
						<a href="/admin/wafer#settings" class="profile-menu-item">
							<${Settings} size=${16} />
							Settings
						</a>
						<div class="profile-menu-divider" />
						<button
							class="profile-menu-item profile-menu-item-danger"
							onClick=${() => { logout(); window.location.href = '/auth/login'; }}
							type="button"
						>
							<${LogOut} size=${16} />
							Log Out
						</button>
					</div>
				` : null}
			</div>
		</nav>
	`;
}
