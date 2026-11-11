import { javascript } from '@codemirror/lang-javascript';
import { html } from '@codemirror/lang-html';
import { css } from '@codemirror/lang-css';
import { json } from '@codemirror/lang-json';
import { markdown } from '@codemirror/lang-markdown';
import { sql } from '@codemirror/lang-sql';
import { php } from '@codemirror/lang-php';
import { python } from '@codemirror/lang-python';
import { StreamLanguage } from '@codemirror/language';
import { yaml } from '@codemirror/legacy-modes/mode/yaml';
import type { Extension } from '@codemirror/state';

export function getLanguageExtension(filename: string): Extension {
  // Handle compound extensions like .html.twig
  const lower = filename.toLowerCase();
  if (lower.endsWith('.html.twig') || lower.endsWith('.twig')) {
    return html();
  }
  const ext = filename.split('.').pop()?.toLowerCase() ?? '';
  switch (ext) {
    case 'js':  return javascript();
    case 'mjs': return javascript();
    case 'cjs': return javascript();
    case 'ts':  return javascript({ typescript: true });
    case 'jsx': return javascript({ jsx: true });
    case 'tsx': return javascript({ jsx: true, typescript: true });
    case 'html':return html();
    case 'css': return css();
    case 'json': return json();
    case 'md':  return markdown();
    case 'markdown': return markdown();
    case 'sql': return sql();
    case 'php': return php();
    case 'theme':  return php();
    case 'module': return php();
    case 'install': return php();
    case 'inc':    return php();
    case 'engine': return php();
    case 'twig': return html();
    case 'py': return python();
    case 'yml':  return StreamLanguage.define(yaml);
    case 'yaml': return StreamLanguage.define(yaml);
    default:    return [];
  }
}

export function getLanguageLabel(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() ?? '';
  // Handle compound extensions like .html.twig
  const lower = filename.toLowerCase();
  if (lower.endsWith('.html.twig')) return 'Twig';
  if (lower.endsWith('.twig')) return 'Twig';
  const labels: Record<string, string> = {
    js: 'JavaScript', mjs: 'JavaScript', cjs: 'JavaScript',
    ts: 'TypeScript', jsx: 'JSX', tsx: 'TSX',
    html: 'HTML', css: 'CSS', json: 'JSON',
    md: 'Markdown', markdown: 'Markdown',
    sql: 'SQL', php: 'PHP',
    theme: 'PHP (Drupal)', module: 'PHP (Drupal)', install: 'PHP (Drupal)',
    inc: 'PHP (Drupal)', engine: 'PHP (Drupal)',
    twig: 'Twig', py: 'Python',
    yml: 'YAML', yaml: 'YAML',
  };
  return labels[ext] ?? ext.toUpperCase();
}
