import { html } from '../../htm';
import { Button } from './Button';
import { Download } from 'lucide-preact';

interface ExportButtonProps {
	data: any[];
	filename?: string;
	format?: 'csv' | 'json';
	label?: string;
}

export function ExportButton({ data, filename = 'export', format = 'csv', label = 'Export' }: ExportButtonProps) {
	function handleExport() {
		let content: string;
		let mimeType: string;
		let ext: string;

		if (format === 'json') {
			content = JSON.stringify(data, null, 2);
			mimeType = 'application/json';
			ext = 'json';
		} else {
			if (data.length === 0) return;
			const keys = Object.keys(data[0]);
			const rows = [keys.join(','), ...data.map(row => keys.map(k => {
				const val = String(row[k] ?? '');
				return val.includes(',') || val.includes('"') ? `"${val.replace(/"/g, '""')}"` : val;
			}).join(','))];
			content = rows.join('\n');
			mimeType = 'text/csv';
			ext = 'csv';
		}

		const blob = new Blob([content], { type: mimeType });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${filename}.${ext}`;
		a.click();
		URL.revokeObjectURL(url);
	}

	return html`
		<${Button} variant="secondary" size="sm" icon=${Download} onClick=${handleExport} disabled=${data.length === 0}>
			${label}
		<//>
	`;
}
