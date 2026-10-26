# Batch Rename

A tiny desktop app for renaming many files at once, inspired by
[PowerToys PowerRename](https://learn.microsoft.com/windows/powertoys/powerrename) but standalone and
cross-platform. Built with [Tauri 2](https://v2.tauri.app/), React and
[Fluent UI](https://react.fluentui.dev/).

## Features

- **Search and replace** with plain text or regular expressions, first match or all matches, optionally case sensitive.
- **Apply to** the filename, the extension, or both.
- **Include** files, folders, and/or the items inside added folders.
- **Text formatting**: lowercase, UPPERCASE, Title Case, Capitalize Each Word.
- **Replace patterns**: counters, random strings/UUIDs and the file's date and time (see below).
- **Live preview** with per-item checkboxes and a filter to show only the items that will change.
  Problems (invalid names, duplicates, existing files) are flagged before anything is renamed.
- **Safe renaming**: existing files are never overwritten, swaps such as `a → b, b → a` work,
  folder contents are renamed before their folders, and **Undo** restores the previous names.
- **Ways to add items**: drag and drop, *Add files* / *Add folder*, command-line arguments, and
  *Rename with Batch Rename* in the File Explorer context menu (Windows, see below).
- **Portable**: a single executable, no installer. Optionally add it to the Start menu.
- English and Simplified Chinese, light and dark theme, recently used search/replace suggestions.

Keyboard: <kbd>Ctrl</kbd>+<kbd>Enter</kbd> renames.

## Windows integration

Everything is off by default and can be turned on in **Settings**:

- **Show in the Start menu** adds a shortcut to your Start menu (for your account only).
- **Show "Rename with Batch Rename" in the File Explorer context menu** puts the entry at the top
  of the Windows 11 menu (on Windows 10, in the regular menu). Windows 11 only allows that for
  signed app packages, so the app creates a small package for its menu handler, signs it with a
  certificate made on your PC, and asks **once** for administrator permission to trust that
  certificate. The certificate's private key is deleted right after signing, so it can't be used
  to sign anything else. Windows 10 needs no administrator permission.

When you move the app, the Start menu shortcut and the menu entry follow it the next time you
start it. To remove the app, turn both settings off, then delete the executable. Its settings are
in `%LOCALAPPDATA%\io.github.miniwangdali.batch-rename`. On Windows 11 the (now useless)
certificate stays in the computer's *Trusted People* store; you can remove
*Batch Rename Context Menu* there with `certlm.msc` if you like.

The app needs the Microsoft Edge WebView2 Runtime, which is part of Windows 11 and up-to-date
Windows 10.

## Replace patterns

| Pattern | Meaning |
| --- | --- |
| `${}` | Counter starting at 0 (needs *Enumerate items*) |
| `${start=10,increment=5,padding=4}` | Counter with custom start, step and zero padding (`;` also works as separator) |
| `${rstringalnum=N}` / `${rstringalpha=N}` / `${rstringdigit=N}` | N random letters+digits / letters / digits (needs *Random values*) |
| `${ruuidv4}` | Random UUID v4 (needs *Random values*) |
| `$YYYY` `$YY` `$Y` | Year: 4 digits, 2 digits, last digit |
| `$MMMM` `$MMM` `$MM` `$M` | Month: name, short name, 2 digits, number |
| `$DDDD` `$DDD` `$DD` `$D` | Day of week name, short name; day of month 2 digits, number |
| `$hh` `$h` `$mm` `$m` `$ss` `$s` | Hours (24h), minutes, seconds |
| `$fff` `$ff` `$f` | Milliseconds |
| `$1` … `$99`, `$<name>`, `$&` | Regex groups and whole match (regex mode only) |
| `$$` | A literal `$` |

Dates come from the file's creation time by default; you can switch to the last modified time.
The buttons inside the *Search for* and *Replace with* boxes list common regex examples and these
patterns; clicking one inserts it.

### Regex examples

| Search for | Replace with | Result |
| --- | --- | --- |
| `^` | `2024_` | Adds a prefix |
| `(\d{2})-(\d{2})-(\d{4})` | `$3-$2-$1` | `29-03-2020` → `2020-03-29` |
| `.*` | `photo_${start=1,padding=3}` | `photo_001`, `photo_002`, … (apply to *Filename only*) |
| `\s*\(\d+\)` | *(empty)* | Removes `(1)`, `(2)` suffixes |

Regular expressions use Rust's [`fancy-regex`](https://docs.rs/fancy-regex) (Perl/ECMAScript-like,
including lookaround and backreferences).

## Development

Prerequisites: [Node.js](https://nodejs.org/) 20+, [Rust](https://rustup.rs/) (stable) and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. On Windows the build
also uses `makeappx.exe` from the Windows SDK (installed with the Visual Studio C++ build tools; set
`MAKEAPPX` to use another copy).

```sh
npm install
npm run tauri dev     # run the app with hot reload
npm test              # Rust unit tests
npm run tauri build   # release build: src-tauri/target/release/batch-rename.exe
```

Project layout:

- `src/` – React UI (`Workspace.tsx` is the main screen, `locales/` holds the translations).
- `src-tauri/src/engine/` – the rename engine: replace patterns, text formatting, regex.
- `src-tauri/src/session.rs` – item list, preview with conflict detection, rename and undo.
- `src-tauri/src/fsops.rs` – non-overwriting renames and the batch executor.
- `src-tauri/src/integration/` – Start menu shortcut and File Explorer context menu (classic and
  top-level: package signing, certificate trust, registration).
- `src-tauri/context-menu/` – the Windows 11 context menu handler (a small COM DLL) and its package
  manifest. `src-tauri/build.rs` builds both and embeds them in the executable.

## License

[MIT](LICENSE.md)
