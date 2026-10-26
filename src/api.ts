import { invoke } from "@tauri-apps/api/core";

export interface Item {
  id: number;
  name: string;
  depth: number;
  isDir: boolean;
  parent?: number;
  /** Only set for top-level items. */
  dir?: string;
}

export interface AddReport {
  added: number;
  truncated: boolean;
  missing: string[];
}

export interface ItemsResponse {
  items: Item[];
  report: AddReport;
}

export type ApplyTo = "name" | "extension" | "both";
export type TextCase = "none" | "lower" | "upper" | "title" | "capitalize";
export type TimeSource = "created" | "modified";

export interface RenameOptions {
  search: string;
  replace: string;
  useRegex: boolean;
  matchAll: boolean;
  caseSensitive: boolean;
  applyTo: ApplyTo;
  includeFiles: boolean;
  includeFolders: boolean;
  includeSubfolders: boolean;
  textCase: TextCase;
  enumerate: boolean;
  random: boolean;
  timeSource: TimeSource;
}

export interface DateNames {
  months: string[];
  monthsShort: string[];
  days: string[];
  daysShort: string[];
}

export type ErrorCode =
  | "empty"
  | "illegalChars"
  | "reserved"
  | "trailing"
  | "tooLong"
  | "duplicate"
  | "exists"
  | "regex";

export interface PreviewEntry {
  id: number;
  newName: string;
  error?: ErrorCode;
  detail?: string;
}

export interface Preview {
  version: number;
  entries: PreviewEntry[];
  renamable: number;
  errors: number;
}

export interface Failure {
  id: number;
  name: string;
  message: string;
}

export interface RenameResponse {
  items: Item[];
  report: { renamed: number; failures: Failure[]; canUndo: boolean };
}

export const api = {
  addPaths: (paths: string[]) => invoke<ItemsResponse>("add_paths", { paths }),
  addPending: () => invoke<ItemsResponse>("add_pending"),
  clearItems: () => invoke<void>("clear_items"),
  preview: (options: RenameOptions & { dateNames: DateNames }, deselected: number[]) =>
    invoke<Preview>("preview", { options, deselected }),
  rename: (version: number) => invoke<RenameResponse>("rename", { version }),
  undo: () => invoke<RenameResponse>("undo"),
  shellIntegration: () => invoke<boolean | null>("shell_integration"),
  setShellIntegration: (enabled: boolean, label: string) =>
    invoke<void>("set_shell_integration", { enabled, label }),
};
