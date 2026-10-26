import type { RenameOptions } from "./api";

export type Language = "system" | "en" | "zh-CN";
export type Theme = "system" | "light" | "dark";

export interface Settings {
  language: Language;
  theme: Theme;
  /** Restore option flags (not the search/replace text) on launch. */
  restoreOptions: boolean;
  /** Offer recently used search/replace text as suggestions. */
  rememberRecent: boolean;
  maxRecent: number;
  /** Fill in the last used search/replace text on launch. */
  fillLastUsed: boolean;
  closeAfterRename: boolean;
}

export interface Recent {
  search: string[];
  replace: string[];
  last?: { search: string; replace: string };
}

export const defaultSettings: Settings = {
  language: "system",
  theme: "system",
  restoreOptions: true,
  rememberRecent: true,
  maxRecent: 10,
  fillLastUsed: false,
  closeAfterRename: false,
};

export const defaultOptions: RenameOptions = {
  search: "",
  replace: "",
  useRegex: false,
  matchAll: false,
  caseSensitive: false,
  applyTo: "name",
  includeFiles: true,
  includeFolders: true,
  includeSubfolders: true,
  textCase: "none",
  enumerate: true,
  random: false,
  timeSource: "created",
};

const KEYS = { settings: "settings", options: "options", recent: "recent" } as const;

function load<T extends object>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage is best effort.
  }
}

export const loadSettings = () => load(KEYS.settings, defaultSettings);
export const saveSettings = (s: Settings) => save(KEYS.settings, s);
export const loadRecent = () => load<Recent>(KEYS.recent, { search: [], replace: [] });
export const saveRecent = (r: Recent) => save(KEYS.recent, r);

export function saveOptions(options: RenameOptions) {
  const { search: _search, replace: _replace, ...flags } = options;
  save(KEYS.options, flags);
}

export function initialOptions(settings: Settings, recent: Recent): RenameOptions {
  const flags = settings.restoreOptions ? load(KEYS.options, defaultOptions) : defaultOptions;
  const text =
    settings.fillLastUsed && recent.last ? recent.last : { search: "", replace: "" };
  return { ...flags, ...text };
}

export function pushRecent(recent: Recent, search: string, replace: string, max: number): Recent {
  const push = (list: string[], value: string) =>
    value ? [value, ...list.filter((v) => v !== value)].slice(0, max) : list.slice(0, max);
  return {
    search: push(recent.search, search),
    replace: push(recent.replace, replace),
    last: { search, replace },
  };
}
