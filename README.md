# ctlgr

[![CI](https://github.com/jezumbro/ctlgr-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/jezumbro/ctlgr-cli/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/jezumbro/ctlgr-cli/graph/badge.svg)](https://codecov.io/gh/jezumbro/ctlgr-cli)

A CLI for creating and searching `.html` catalog files, designed as an agent-friendly interface.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/jezumbro/ctlgr-cli/main/install.sh | sh
```

Installs a pre-built binary to `~/.local/bin`. Supports macOS (Intel + Apple Silicon) and
Linux (x86\_64 + arm64).

**Options:**

```sh
# Specific version
VERSION=0.1.0 curl -fsSL .../install.sh | sh

# Custom install directory
INSTALL_DIR=/usr/local/bin curl -fsSL .../install.sh | sh
```

**From source** (requires Rust):

```sh
cargo install --git https://github.com/jezumbro/ctlgr-cli
```

## Usage

```
ctlgr <command> [flags]
```

### search

Search HTML catalog files using CSS selectors.

```sh
ctlgr search [<selector>] [flags]
```

| Flag                    | Short | Description                                        | Default     |
| ----------------------- | ----- | -------------------------------------------------- | ----------- |
| `--file <file>`         | `-f`  | HTML file(s) to search (repeatable)                | config path |
| `--tag <tag>`           | `-t`  | Filter by tag name                                 | —           |
| `--attr <name[=value]>` | `-a`  | Filter by attribute presence or value (repeatable) | —           |
| `--text <pattern>`      |       | Case-insensitive substring match on element text   | —           |
| `--json <fields>`       |       | Output JSON: `tag,attrs,text,html,path`            | —           |
| `--md`                  |       | Output results as Markdown                         | —           |
| `--jq <expr>`           | `-q`  | Filter JSON output with a jq expression            | —           |
| `--limit <n>`           | `-L`  | Maximum results                                    | `30`        |

When `--file` is omitted, ctlgr searches all `*.html` and `*.md` files under the configured catalog
directory. Files matching any `excluded` pattern are skipped.

```sh
# All links across the configured catalog
ctlgr search "a"

# Text search: headings mentioning "config" with enclosing section context
ctlgr search "h2,h3" --text config

# Full article content as Markdown
ctlgr search "article" --text init --md

# Nav links as JSON
ctlgr search "nav a" --json tag,attrs,text --file page.html
```

### config

Manage the catalog directory and config files.

```sh
ctlgr config <subcommand>
```

| Subcommand            | Description                                                  |
| --------------------- | ------------------------------------------------------------ |
| `init <path>`         | Create `.ctlgr` in the current directory with the given path |
| `init --local <path>` | Create `.ctlgr.local` instead (gitignored, higher priority)  |
| `list`                | Show the resolved catalog path                               |

Config resolution walks up from the current directory (first match wins for `path`/`lint`; all
levels are merged for `excluded`):

1. `.ctlgr.local` — personal overrides, not committed
2. `.ctlgr` — per-directory config, committed
3. `~/.ctlgr-cli/settings.json` — global fallback
4. `~/.ctlgr-cli/catalog/` — hardcoded default

To change the path, delete `.ctlgr` and re-run `config init`. Legacy `.ctlgr.json` files are
auto-migrated to `.ctlgr` on first run.

**Config file schema** (`.ctlgr`):

```json
{
  "path": "/Users/you/catalog",
  "excluded": ["AGENTS\\.md", "drafts/"],
  "lint": {
    "rules": ["no-style-blocks", "no-inline-styles", "prefer-html"]
  }
}
```

`excluded` is an array of regex patterns matched against the full file path. Patterns are merged
across all config levels, so global personal exclusions work alongside project ones.

### lint

Lint catalog HTML files for style violations.

```sh
ctlgr lint [flags]
```

| Flag            | Short | Description                                      |
| --------------- | ----- | ------------------------------------------------ |
| `--file <file>` | `-f`  | Files to lint (repeatable; defaults to catalog)  |
| `--write`       |       | Fix violations in place; convert `.md` → `.html` |

Rules (all enabled by default; configure via `lint.rules` in the config file):

| Rule               | Description                                |
| ------------------ | ------------------------------------------ |
| `no-style-blocks`  | `<style>` blocks are not allowed           |
| `no-inline-styles` | `style="..."` attributes are not allowed   |
| `prefer-html`      | `.md` files should be converted to `.html` |

### update

Check for a newer version and upgrade in-place.

```sh
ctlgr update
```

A background check also runs automatically on each command (at most once per 24 hours), printing a
notice to stderr when a new version is available.

## Development

```sh
cargo build
cargo test
cargo bench --bench startup -- --test   # run startup benchmarks in test mode
```

### Dev tools

Install before contributing (declared in `[package.metadata.tools]`):

```sh
cargo install committed   # conventional commit enforcement (commit-msg hook)
cargo install dprint      # markdown formatting (pre-commit hook)
```

### Test layout

Tests mirror the source tree. Each `src/<module>.rs` has a corresponding `tests/<module>.rs` that
tests its public API. `tests/integration.rs` is the exception — it drives the compiled binary
end-to-end via `assert_cmd`.

| Test file              | What it covers                                  |
| ---------------------- | ----------------------------------------------- |
| `tests/settings.rs`    | Config discovery, load/save, path expansion     |
| `tests/search.rs`      | Selector building, filtering, matching, output  |
| `tests/update.rs`      | Version comparison, cooldown, self-update logic |
| `tests/integration.rs` | Full CLI: all commands and flags via binary     |

Commits must follow [Conventional Commits](https://www.conventionalcommits.org/) — enforced via a
`commit-msg` hook. `.md` files are formatted by `dprint` on `pre-commit`.

See [AGENTS.md](AGENTS.md) for the full agent interface reference.
