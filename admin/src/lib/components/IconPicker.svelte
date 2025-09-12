<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { 
		// Business & Commerce
		Building2, Store, Briefcase, Factory, Warehouse, ShoppingBag, Coffee,
		ShoppingCart, Package2, Box, Gift, Tag, Barcode, QrCode, Archive, Receipt,
		
		// Infrastructure & Locations
		Home, Hotel, Building, School, MapPin, Globe,
		
		// Transportation
		Truck, Car, Plane, Ship, Train, Bike, Bus, Rocket, Luggage, Anchor,
		
		// Nature & Outdoor
		Mountain, Trees, Sun, Moon, Cloud, Wind, Droplet, Flame,
		
		// Technology
		Server, Database, HardDrive, Cpu, Smartphone, Laptop, Monitor, Printer,
		Wifi, Download, Upload, FileText, FileCode, FileArchive, FolderOpen,
		
		// Media & Entertainment
		Gamepad2, Music, Palette, Camera, Film, Radio, Tv, Video, Headphones,
		Mic, Speaker, Volume2,
		
		// People & Organizations
		Users, UserCheck, Crown, Shield,
		
		// Finance & Payment
		CreditCard, Banknote, Coins, Wallet, PiggyBank, TrendingUp, BarChart3,
		PieChart, Activity, DollarSign,
		
		// Premium & Rewards
		Diamond, Gem, Trophy, Medal, Award, Ribbon, Star, Heart, Zap,
		
		// Time & Schedule
		Clock, Calendar, Watch, Timer, Hourglass, Bell,
		
		// Tools & Hardware
		Wrench, Hammer, PaintBucket, Brush, Settings,
		
		// Food & Beverage
		Coffee as Coffee2, Pizza, Cake, Cookie, Apple, Cherry, Grape, Carrot,
		Salad, Beer, Wine, Milk, Popcorn, IceCream2,
		
		// Fashion & Accessories
		Shirt, Footprints, Umbrella, Backpack,
		
		// Abstract & Misc
		Target, Flag, Key, Battery, Plug, Lightbulb, Book, Newspaper,
		ScrollText, FileCheck, Ticket, Compass, Navigation, Package,
		Layers, Code, CheckCircle, XCircle, Plus, Edit2, Trash2,
		Search, Filter, ChevronRight, ArrowLeft, X
	} from 'lucide-svelte';
	
	export let value = '';
	export let showLabel = true;
	export let placeholder = 'Select an icon';
	export let disabled = false;
	
	const dispatch = createEventDispatcher();
	
	let showPicker = false;
	let searchQuery = '';
	let selectedCategory = 'all';
	
	// Icon categories with their icons
	const iconCategories = {
		'Business & Commerce': [
			{ value: 'building', label: 'Building', icon: Building2 },
			{ value: 'store', label: 'Store', icon: Store },
			{ value: 'briefcase', label: 'Business', icon: Briefcase },
			{ value: 'factory', label: 'Factory', icon: Factory },
			{ value: 'warehouse', label: 'Warehouse', icon: Warehouse },
			{ value: 'shopping', label: 'Shop', icon: ShoppingBag },
			{ value: 'coffee', label: 'Cafe', icon: Coffee },
			{ value: 'cart', label: 'Cart', icon: ShoppingCart },
			{ value: 'package', label: 'Package', icon: Package2 },
			{ value: 'box', label: 'Box', icon: Box },
			{ value: 'gift', label: 'Gift', icon: Gift },
			{ value: 'tag', label: 'Tag', icon: Tag },
			{ value: 'barcode', label: 'Barcode', icon: Barcode },
			{ value: 'qrcode', label: 'QR Code', icon: QrCode },
			{ value: 'archive', label: 'Archive', icon: Archive },
			{ value: 'receipt', label: 'Receipt', icon: Receipt }
		],
		'Infrastructure': [
			{ value: 'home', label: 'Home', icon: Home },
			{ value: 'hotel', label: 'Hotel', icon: Hotel },
			{ value: 'building', label: 'Building', icon: Building },
			{ value: 'building2', label: 'Office', icon: Building2 },
			{ value: 'school', label: 'School', icon: School },
			{ value: 'mappin', label: 'Location', icon: MapPin },
			{ value: 'globe', label: 'Global', icon: Globe }
		],
		'Transportation': [
			{ value: 'truck', label: 'Truck', icon: Truck },
			{ value: 'car', label: 'Car', icon: Car },
			{ value: 'plane', label: 'Plane', icon: Plane },
			{ value: 'ship', label: 'Ship', icon: Ship },
			{ value: 'train', label: 'Train', icon: Train },
			{ value: 'bike', label: 'Bike', icon: Bike },
			{ value: 'bus', label: 'Bus', icon: Bus },
			{ value: 'rocket', label: 'Rocket', icon: Rocket },
			{ value: 'luggage', label: 'Luggage', icon: Luggage },
			{ value: 'anchor', label: 'Anchor', icon: Anchor }
		],
		'Nature': [
			{ value: 'mountain', label: 'Mountain', icon: Mountain },
			{ value: 'trees', label: 'Forest', icon: Trees },
			{ value: 'sun', label: 'Sun', icon: Sun },
			{ value: 'moon', label: 'Moon', icon: Moon },
			{ value: 'cloud', label: 'Cloud', icon: Cloud },
			{ value: 'wind', label: 'Wind', icon: Wind },
			{ value: 'droplet', label: 'Water', icon: Droplet },
			{ value: 'flame', label: 'Fire', icon: Flame }
		],
		'Technology': [
			{ value: 'server', label: 'Server', icon: Server },
			{ value: 'database', label: 'Database', icon: Database },
			{ value: 'harddrive', label: 'Storage', icon: HardDrive },
			{ value: 'cpu', label: 'Processor', icon: Cpu },
			{ value: 'smartphone', label: 'Mobile', icon: Smartphone },
			{ value: 'laptop', label: 'Laptop', icon: Laptop },
			{ value: 'monitor', label: 'Desktop', icon: Monitor },
			{ value: 'printer', label: 'Printer', icon: Printer },
			{ value: 'wifi', label: 'Network', icon: Wifi },
			{ value: 'download', label: 'Download', icon: Download },
			{ value: 'upload', label: 'Upload', icon: Upload },
			{ value: 'file', label: 'File', icon: FileText },
			{ value: 'code', label: 'Code', icon: FileCode },
			{ value: 'archive2', label: 'Zip', icon: FileArchive },
			{ value: 'folder', label: 'Folder', icon: FolderOpen }
		],
		'Media': [
			{ value: 'gamepad', label: 'Gaming', icon: Gamepad2 },
			{ value: 'music', label: 'Music', icon: Music },
			{ value: 'palette', label: 'Art', icon: Palette },
			{ value: 'camera', label: 'Camera', icon: Camera },
			{ value: 'film', label: 'Film', icon: Film },
			{ value: 'radio', label: 'Radio', icon: Radio },
			{ value: 'tv', label: 'TV', icon: Tv },
			{ value: 'video', label: 'Video', icon: Video },
			{ value: 'headphones', label: 'Headphones', icon: Headphones },
			{ value: 'mic', label: 'Microphone', icon: Mic },
			{ value: 'speaker', label: 'Speaker', icon: Speaker },
			{ value: 'volume', label: 'Volume', icon: Volume2 }
		],
		'People': [
			{ value: 'users', label: 'Team', icon: Users },
			{ value: 'usercheck', label: 'Members', icon: UserCheck },
			{ value: 'crown', label: 'Premium', icon: Crown },
			{ value: 'shield', label: 'Security', icon: Shield }
		],
		'Finance': [
			{ value: 'creditcard', label: 'Card', icon: CreditCard },
			{ value: 'banknote', label: 'Cash', icon: Banknote },
			{ value: 'coins', label: 'Coins', icon: Coins },
			{ value: 'wallet', label: 'Wallet', icon: Wallet },
			{ value: 'piggybank', label: 'Savings', icon: PiggyBank },
			{ value: 'trending', label: 'Growth', icon: TrendingUp },
			{ value: 'chart', label: 'Analytics', icon: BarChart3 },
			{ value: 'piechart', label: 'Pie Chart', icon: PieChart },
			{ value: 'activity', label: 'Activity', icon: Activity },
			{ value: 'dollar', label: 'Dollar', icon: DollarSign }
		],
		'Awards': [
			{ value: 'diamond', label: 'Diamond', icon: Diamond },
			{ value: 'gem', label: 'Gem', icon: Gem },
			{ value: 'trophy', label: 'Trophy', icon: Trophy },
			{ value: 'medal', label: 'Medal', icon: Medal },
			{ value: 'award', label: 'Award', icon: Award },
			{ value: 'ribbon', label: 'Ribbon', icon: Ribbon },
			{ value: 'star', label: 'Star', icon: Star },
			{ value: 'heart', label: 'Favorite', icon: Heart },
			{ value: 'zap', label: 'Energy', icon: Zap }
		],
		'Time': [
			{ value: 'clock', label: 'Clock', icon: Clock },
			{ value: 'calendar', label: 'Calendar', icon: Calendar },
			{ value: 'watch', label: 'Watch', icon: Watch },
			{ value: 'timer', label: 'Timer', icon: Timer },
			{ value: 'hourglass', label: 'Hourglass', icon: Hourglass },
			{ value: 'bell', label: 'Bell', icon: Bell }
		],
		'Tools': [
			{ value: 'wrench', label: 'Wrench', icon: Wrench },
			{ value: 'hammer', label: 'Hammer', icon: Hammer },
			{ value: 'paintbucket', label: 'Paint', icon: PaintBucket },
			{ value: 'brush', label: 'Brush', icon: Brush },
			{ value: 'settings', label: 'Settings', icon: Settings },
		],
		'Food': [
			{ value: 'coffee2', label: 'Coffee', icon: Coffee2 },
			{ value: 'pizza', label: 'Pizza', icon: Pizza },
			{ value: 'cake', label: 'Cake', icon: Cake },
			{ value: 'cookie', label: 'Cookie', icon: Cookie },
			{ value: 'apple', label: 'Apple', icon: Apple },
			{ value: 'cherry', label: 'Cherry', icon: Cherry },
			{ value: 'grape', label: 'Grape', icon: Grape },
			{ value: 'carrot', label: 'Carrot', icon: Carrot },
			{ value: 'salad', label: 'Salad', icon: Salad },
			{ value: 'beer', label: 'Beer', icon: Beer },
			{ value: 'wine', label: 'Wine', icon: Wine },
			{ value: 'milk', label: 'Milk', icon: Milk },
			{ value: 'popcorn', label: 'Popcorn', icon: Popcorn },
			{ value: 'icecream', label: 'Ice Cream', icon: IceCream2 }
		],
		'Fashion': [
			{ value: 'shirt', label: 'Clothing', icon: Shirt },
			{ value: 'footprints', label: 'Footwear', icon: Footprints },
			{ value: 'umbrella', label: 'Umbrella', icon: Umbrella },
			{ value: 'backpack', label: 'Backpack', icon: Backpack }
		],
		'Misc': [
			{ value: 'target', label: 'Target', icon: Target },
			{ value: 'flag', label: 'Flag', icon: Flag },
			{ value: 'key', label: 'Key', icon: Key },
			{ value: 'battery', label: 'Battery', icon: Battery },
			{ value: 'plug', label: 'Plugin', icon: Plug },
			{ value: 'lightbulb', label: 'Idea', icon: Lightbulb },
			{ value: 'book', label: 'Book', icon: Book },
			{ value: 'newspaper', label: 'News', icon: Newspaper },
			{ value: 'scroll', label: 'Document', icon: ScrollText },
			{ value: 'certificate', label: 'Certificate', icon: FileCheck },
			{ value: 'ticket', label: 'Ticket', icon: Ticket },
			{ value: 'compass', label: 'Compass', icon: Compass },
			{ value: 'navigation', label: 'Navigate', icon: Navigation },
			{ value: 'package2', label: 'Parcel', icon: Package },
			{ value: 'layers', label: 'Layers', icon: Layers },
			{ value: 'code2', label: 'Source', icon: Code }
		]
	};
	
	// Flatten all icons for searching
	const allIcons = Object.values(iconCategories).flat();
	
	// Find icon by value
	function getIconByValue(val: string) {
		return allIcons.find(icon => icon.value === val);
	}
	
	// Get selected icon
	$: selectedIcon = getIconByValue(value);
	
	// Filter icons based on search and category
	$: filteredIcons = (() => {
		let icons = selectedCategory === 'all' 
			? allIcons 
			: iconCategories[selectedCategory] || [];
		
		if (searchQuery) {
			icons = icons.filter(icon => 
				icon.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
				icon.value.toLowerCase().includes(searchQuery.toLowerCase())
			);
		}
		
		return icons;
	})();
	
	// Handle icon selection
	function selectIcon(icon: any) {
		value = icon.value;
		showPicker = false;
		searchQuery = '';
		dispatch('change', icon);
	}
	
	// Handle clicking outside
	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.icon-picker-container')) {
			showPicker = false;
		}
	}
</script>

<svelte:window on:click={handleClickOutside} />

<div class="icon-picker-container">
	<button
		type="button"
		class="icon-picker-trigger"
		class:disabled
		disabled={disabled}
		on:click|stopPropagation={() => showPicker = !showPicker}
	>
		{#if selectedIcon}
			<div class="selected-icon">
				<svelte:component this={selectedIcon.icon} size={20} />
				{#if showLabel}
					<span>{selectedIcon.label}</span>
				{/if}
			</div>
		{:else}
			<span class="placeholder">{placeholder}</span>
		{/if}
		<span class="chevron" class:rotated={showPicker}>
			<ChevronRight size={16} />
		</span>
	</button>
	
	{#if showPicker && !disabled}
		<div class="icon-picker-dropdown" on:click|stopPropagation>
			<div class="picker-header">
				<input
					type="text"
					class="search-input"
					placeholder="Search icons..."
					bind:value={searchQuery}
					on:click|stopPropagation
				/>
				<select 
					class="category-select" 
					bind:value={selectedCategory}
					on:click|stopPropagation
				>
					<option value="all">All Categories</option>
					{#each Object.keys(iconCategories) as category}
						<option value={category}>{category}</option>
					{/each}
				</select>
			</div>
			
			<div class="icons-grid">
				{#each filteredIcons as icon}
					<button
						type="button"
						class="icon-option"
						class:selected={value === icon.value}
						on:click={() => selectIcon(icon)}
						title={icon.label}
					>
						<svelte:component this={icon.icon} size={24} />
						<span class="icon-label">{icon.label}</span>
					</button>
				{/each}
				
				{#if filteredIcons.length === 0}
					<div class="no-results">
						No icons found
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.icon-picker-container {
		position: relative;
		width: 100%;
	}
	
	.icon-picker-trigger {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 0.875rem;
	}
	
	.icon-picker-trigger:hover:not(.disabled) {
		border-color: #06b6d4;
	}
	
	.icon-picker-trigger:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}
	
	.icon-picker-trigger.disabled {
		background: #f9fafb;
		cursor: not-allowed;
		opacity: 0.6;
	}
	
	.selected-icon {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #111827;
	}
	
	.placeholder {
		color: #9ca3af;
	}
	
	.chevron {
		display: flex;
		align-items: center;
		justify-content: center;
		color: #6b7280;
		transition: transform 0.2s;
		flex-shrink: 0;
	}
	
	.chevron.rotated {
		transform: rotate(90deg);
	}
	
	.icon-picker-dropdown {
		position: absolute;
		top: calc(100% + 0.5rem);
		left: 0;
		right: 0;
		max-width: 480px;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
		z-index: 1000;
		overflow: hidden;
	}
	
	.picker-header {
		display: flex;
		gap: 0.5rem;
		padding: 0.75rem;
		border-bottom: 1px solid #e5e7eb;
		background: #f9fafb;
	}
	
	.search-input {
		flex: 1;
		padding: 0.375rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		outline: none;
	}
	
	.search-input:focus {
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}
	
	.category-select {
		padding: 0.375rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		background: white;
		cursor: pointer;
		outline: none;
	}
	
	.category-select:focus {
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}
	
	.icons-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
		gap: 0.5rem;
		padding: 0.75rem;
		max-height: 400px;
		overflow-y: auto;
	}
	
	.icon-option {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
		padding: 0.75rem 0.5rem;
		border: 1px solid transparent;
		border-radius: 0.375rem;
		background: white;
		cursor: pointer;
		transition: all 0.2s;
		color: #374151;
	}
	
	.icon-option:hover {
		background: #f3f4f6;
		border-color: #e5e7eb;
	}
	
	.icon-option.selected {
		background: #ecfdf5;
		border-color: #06b6d4;
		color: #06b6d4;
	}
	
	.icon-label {
		font-size: 0.75rem;
		text-align: center;
		word-break: break-word;
	}
	
	.no-results {
		grid-column: 1 / -1;
		padding: 2rem;
		text-align: center;
		color: #6b7280;
	}
	
	/* Scrollbar styling */
	.icons-grid::-webkit-scrollbar {
		width: 6px;
	}
	
	.icons-grid::-webkit-scrollbar-track {
		background: #f3f4f6;
	}
	
	.icons-grid::-webkit-scrollbar-thumb {
		background: #d1d5db;
		border-radius: 3px;
	}
	
	.icons-grid::-webkit-scrollbar-thumb:hover {
		background: #9ca3af;
	}
</style>