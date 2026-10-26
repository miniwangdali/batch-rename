# Changelog

All notable changes to this project will be documented in this file.

## 0.3.0 (unreleased)

Rewritten from scratch as a small PowerToys PowerRename–style renamer.

### Features

* Search and replace with plain text or regular expressions (lookarounds, `$1`, `$<name>`, `$&`), first or all occurrences, optional case sensitivity.
* Apply to filename, extension or both; include or exclude files, folders and items inside added folders.
* Text formatting: lowercase, UPPERCASE, Title Case, Capitalize Each Word.
* Replace patterns: `${start=…,increment=…,padding=…}` counters, random strings and UUIDs, file date/time (`$YYYY`, `$MM`, `$DD`, …).
* Live preview with per-item selection, "only renamed" filter, and conflict detection (invalid names, duplicates, existing files).
* Safe renaming: never overwrites, handles swaps and chains, renames folder contents before folders, one-click undo.
* Drag and drop, add files/folders and command-line paths (very large selections are passed through a list file).
* Portable single executable (no installer) with an optional Start menu shortcut.
* File Explorer context menu entry: at the top of the Windows 11 menu through a locally signed sparse package (one administrator prompt), or in the regular menu on Windows 10.
* English and Simplified Chinese UI, light/dark theme, recent search/replace suggestions.

### Under the hood

* Tauri 1 beta → Tauri 2; Create React App + MUI + Recoil → Vite + React 19 + Fluent UI v9; yarn → npm.
* Rename engine moved to Rust with unit tests.

## [0.2.0](https://github.com/miniwangdali/batch-rename/compare/v0.1.0...v0.2.0) (2022-01-01)


### Features

* **fillup:** Allow customized number of fill-up digits. ([8fadc07](https://github.com/miniwangdali/batch-rename/commit/8fadc072fc29d7f9232342a63d9f648a91318f40))

## [0.1.0](https://github.com/miniwangdali/batch-rename/compare/v0.1.0-beta...v0.1.0) (2021-12-11)
