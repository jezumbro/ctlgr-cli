# Settings Refactor Spec

## Problem

The current config system has two file names (`.ctlgr.json` and `.ctlgr.local.json`), an array `paths` field that allows registering multiple directories, and a global path at `~/.ctlgr-cli/settings.json`. The dual-file approach adds discovery complexity, and multi-path support goes against the per-directory ownership model: each directory should declare exactly one catalog root.

## Goals

1. One config file name: `.ctlgr` per directory
2. Single `path` string instead of a `paths` array — first found wins
3. Implicit default: `~/.ctlgr-cli/notes/` when no path is configured anywhere
4. Drop `config init --local` and the `--local` flag

## Config file

**Name**: `.ctlgr` (no extension; JSON format)

**Schema**:

```json
{
  "path": "/path/to/catalog",
  "lint": {
    "rules": ["no-style-blocks", "no-inline-styles", "prefer-html"]
  }
}
```

`path` is optional. Omitting it causes discovery to fall through to the next level.

## Discovery — first `path` found wins

1. `.ctlgr` in CWD, then each ancestor directory in turn
2. `~/.ctlgr-cli/settings.json`
3. `~/.ctlgr-cli/notes/` (hardcoded fallback — always available)

The search stops at the first config file that has `path` set. A `.ctlgr` with no `path` field is valid (e.g. lint-only config) but does not satisfy the path resolution.

## `src/settings.rs` changes

| Current                                                            | New                                                                 |
| ------------------------------------------------------------------ | ------------------------------------------------------------------- |
| `paths: Vec<String>`                                               | `path: Option<String>`                                              |
| `find_config_from()` checks `[".ctlgr.local.json", ".ctlgr.json"]` | checks `[".ctlgr"]` only                                            |
| `local_config_path_from(cwd, local: bool) -> PathBuf`              | `config_path_from(cwd) -> PathBuf`                                  |
| `local_config_path(local: bool) -> Result<PathBuf>`                | `config_path() -> Result<PathBuf>`                                  |
| `expand_paths(settings) -> Result<Vec<String>>`                    | `resolve_path(settings) -> PathBuf`                                 |
| _(new)_                                                            | `default_notes_dir() -> PathBuf` → `~/.ctlgr-cli/notes/`            |
| `load()`                                                           | unchanged signature; callers use `resolve_path` for the catalog dir |

`resolve_path` returns the configured path if set, otherwise `default_notes_dir()`. Callers glob `{dir}/**/*.{html,md}` from that single root.

## `src/main.rs` — config subcommand changes

| Subcommand   | Change                                            |
| ------------ | ------------------------------------------------- |
| `init`       | creates `.ctlgr` in CWD; drop `--local` flag      |
| `add <path>` | writes `path` field, replacing any existing value |
| `remove`     | clears the `path` field; no argument needed       |
| `list`       | prints the single resolved path (or the default)  |

## Call-site changes

- `search` / `lint` `resolve_files`: replace `expand_paths` with `resolve_path`, then glob from that directory
- `lint` rule loading: `settings::load()?.lint` — unchanged

## Test changes

### `tests/settings.rs`

- `local_config_path_from_non_local` / `_local_flag` → single `config_path_from_returns_ctlgr`
- `find_config_from_finds_ctlgr_json` → `find_config_from_finds_ctlgr`
- Remove `find_config_from_finds_local_json`, `find_config_from_local_beats_committed`, `find_config_from_exhausts_names_at_level_before_ascending`
- `find_config_from_falls_back_to_global` — stays green (path still contains `.ctlgr-cli`)
- All `paths: vec![...]` → `path: Some(...)`
- `expand_paths_*` tests → `resolve_path_*`
- New: `resolve_path_falls_back_to_notes_dir`

### `tests/integration.rs`

- All `.ctlgr.json` refs → `.ctlgr`
- Remove `search_local_config_takes_priority_over_committed`
- Remove `config_init_local_flag` test
- `config_init_creates_file`: assert `.ctlgr` exists, not `.ctlgr.json`
- `config remove` test: no path argument

## Migration

Existing `.ctlgr.json` and `.ctlgr.local.json` files will not be picked up after this change. No auto-migration. Users should rename to `.ctlgr` or re-run `config init && config add <path>`.
