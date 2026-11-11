export interface Tab {
  id: string;
  path: string;
  filename: string;
  content: string;
  modified: boolean;
}

export interface FsEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

export interface SearchResult {
  path: string;       // relative to project root
  line_number: number;
  line: string;
  match_start: number;
  match_end: number;
}

export interface GitEntry {
  path: string;
  status: 'M' | 'A' | 'D' | 'R' | '?';
}

export interface Session {
  root: string;
  openTabs: Array<{ path: string; filename: string }>;
  activeTabPath: string | null;
}
