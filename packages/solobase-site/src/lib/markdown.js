import { marked } from 'marked';
import hljs from 'highlight.js';
import 'highlight.js/styles/vs2015.css';

marked.setOptions({
  highlight(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      return hljs.highlight(code, { language: lang }).value;
    }
    return hljs.highlightAuto(code).value;
  },
  gfm: true,
  breaks: false,
});

/**
 * Parse front matter from raw markdown string.
 * Returns { data: {title, description, weight, tags}, content: string }
 */
export function parseFrontMatter(raw) {
  const match = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/);
  if (!match) return { data: {}, content: raw };

  const frontMatter = match[1];
  const content = match[2];
  const data = {};

  for (const line of frontMatter.split('\n')) {
    const colonIndex = line.indexOf(':');
    if (colonIndex === -1) continue;
    const key = line.slice(0, colonIndex).trim();
    let value = line.slice(colonIndex + 1).trim();
    // Remove surrounding quotes
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    // Parse arrays like ["a", "b"]
    if (value.startsWith('[') && value.endsWith(']')) {
      try {
        data[key] = JSON.parse(value);
      } catch {
        data[key] = value;
      }
    } else if (!isNaN(Number(value)) && value !== '') {
      data[key] = Number(value);
    } else {
      data[key] = value;
    }
  }

  return { data, content };
}

/**
 * Render markdown to HTML.
 */
export function renderMarkdown(md) {
  return marked.parse(md);
}

/**
 * Extract headings from markdown for table of contents.
 * Returns array of { id, text, level }
 */
export function extractHeadings(md) {
  const headings = [];
  const regex = /^(#{2,4})\s+(.+)$/gm;
  let match;
  while ((match = regex.exec(md)) !== null) {
    const level = match[1].length;
    const text = match[2].trim();
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-');
    headings.push({ id, text, level });
  }
  return headings;
}
