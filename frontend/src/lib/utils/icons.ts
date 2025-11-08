import { 
	// Business & Commerce
	Building2, Store, Briefcase, Factory, Warehouse, ShoppingBag, Coffee,
	ShoppingCart, Package2, Box, Gift, Tag, Barcode, QrCode, Archive, Receipt,
	
	// Infrastructure & Locations
	Home, Hotel, School, Library,
	Landmark, MapPin, Globe, Building,
	
	// Transportation
	Truck, Car, Plane, Ship, Train, Bike, Bus, Rocket, Luggage, Anchor,
	
	// Nature & Outdoor
	Mountain, Trees, Tent, Sun, Moon, Cloud, Wind, Droplet, Flame,
	
	// Technology
	Server, Database, HardDrive, Cpu, Smartphone, Laptop, Monitor, Printer,
	Wifi, Download, Upload, FileText, FileCode, FileArchive, FolderOpen,
	
	// Media & Entertainment
	Gamepad, Music, Palette, Camera, Film, Radio, Tv, Video, Headphones,
	Mic, Speaker, Volume, Joystick, Puzzle,
	
	// People & Organizations
	Users, UserCheck, HeartHandshake, Crown, Shield,
	
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
	Pizza, Cake, Cookie, Apple, Cherry, Grape, Carrot,
	Salad, Beer, Wine, Milk, Popcorn,
	
	// Fashion & Accessories
	Shirt, Footprints, Umbrella, Backpack,
	
	// Abstract & Misc
	Target, Flag, Key, Battery, Plug, Lightbulb, Book, Newspaper,
	ScrollText, FileCheck, Ticket, Compass, Navigation, Package,
	Layers, Code
} from 'lucide-svelte';

// Map of icon names to components
const iconMap: Record<string, any> = {
	// Business & Commerce
	'building': Building2,
	'store': Store,
	'briefcase': Briefcase,
	'factory': Factory,
	'warehouse': Warehouse,
	'shopping': ShoppingBag,
	'coffee': Coffee,
	'cart': ShoppingCart,
	'package': Package2,
	'box': Box,
	'gift': Gift,
	'tag': Tag,
	'barcode': Barcode,
	'qrcode': QrCode,
	'archive': Archive,
	'receipt': Receipt,
	
	// Infrastructure
	'home': Home,
	'hotel': Hotel,
	'school': School,
	'library': Library,
	'landmark': Landmark,
	'mappin': MapPin,
	'globe': Globe,
	'building2': Building,
	
	// Transportation
	'truck': Truck,
	'car': Car,
	'plane': Plane,
	'ship': Ship,
	'train': Train,
	'bike': Bike,
	'bus': Bus,
	'rocket': Rocket,
	'luggage': Luggage,
	'anchor': Anchor,
	
	// Nature
	'mountain': Mountain,
	'trees': Trees,
	'tent': Tent,
	'sun': Sun,
	'moon': Moon,
	'cloud': Cloud,
	'wind': Wind,
	'droplet': Droplet,
	'flame': Flame,
	
	// Technology
	'server': Server,
	'database': Database,
	'harddrive': HardDrive,
	'cpu': Cpu,
	'smartphone': Smartphone,
	'laptop': Laptop,
	'monitor': Monitor,
	'printer': Printer,
	'wifi': Wifi,
	'download': Download,
	'upload': Upload,
	'file': FileText,
	'code': FileCode,
	'archive2': FileArchive,
	'folder': FolderOpen,
	
	// Media
	'gamepad': Gamepad,
	'music': Music,
	'palette': Palette,
	'camera': Camera,
	'film': Film,
	'radio': Radio,
	'tv': Tv,
	'video': Video,
	'headphones': Headphones,
	'mic': Mic,
	'speaker': Speaker,
	'volume': Volume,
	'joystick': Joystick,
	'puzzle': Puzzle,
	
	// People
	'users': Users,
	'usercheck': UserCheck,
	'hearthandshake': HeartHandshake,
	'crown': Crown,
	'shield': Shield,
	
	// Finance
	'creditcard': CreditCard,
	'banknote': Banknote,
	'coins': Coins,
	'wallet': Wallet,
	'piggybank': PiggyBank,
	'trending': TrendingUp,
	'chart': BarChart3,
	'piechart': PieChart,
	'activity': Activity,
	'dollar': DollarSign,
	
	// Awards
	'diamond': Diamond,
	'gem': Gem,
	'trophy': Trophy,
	'medal': Medal,
	'award': Award,
	'ribbon': Ribbon,
	'star': Star,
	'heart': Heart,
	'zap': Zap,
	
	// Time
	'clock': Clock,
	'calendar': Calendar,
	'watch': Watch,
	'timer': Timer,
	'hourglass': Hourglass,
	'bell': Bell,
	
	// Tools
	'wrench': Wrench,
	'hammer': Hammer,
	'paintbucket': PaintBucket,
	'brush': Brush,
	'settings': Settings,
	
	// Food
	'pizza': Pizza,
	'cake': Cake,
	'cookie': Cookie,
	'apple': Apple,
	'cherry': Cherry,
	'grape': Grape,
	'carrot': Carrot,
	'salad': Salad,
	'beer': Beer,
	'wine': Wine,
	'milk': Milk,
	'popcorn': Popcorn,
	
	// Fashion
	'shirt': Shirt,
	'footprints': Footprints,
	'umbrella': Umbrella,
	'backpack': Backpack,
	
	// Misc
	'target': Target,
	'flag': Flag,
	'key': Key,
	'battery': Battery,
	'plug': Plug,
	'lightbulb': Lightbulb,
	'book': Book,
	'newspaper': Newspaper,
	'scroll': ScrollText,
	'certificate': FileCheck,
	'ticket': Ticket,
	'compass': Compass,
	'navigation': Navigation,
	'package2': Package,
	'layers': Layers,
	'code2': Code
};

// Get icon component by name
export function getIconComponent(iconName?: string) {
	if (!iconName) return Building2; // Default icon
	return iconMap[iconName] || Building2;
}

// Export the icon map for other uses
export { iconMap };