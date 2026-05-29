# ctlgr-cli — .html Agent Interface

`ctlgr` is a CLI for creating and searching `.html` catalog files.

## Automatic skill triggers

**Adding or editing anything in `ctlgr-docs/`**: always follow `.claude/commands/add-doc.md` — run the `ctlgr search` discovery step first before writing or modifying any file.

## Commands

### search

Search HTML files using CSS selectors.

```
ctlgr search [<selector>] [flags]
```

**Flags**

| Flag                    | Short | Description                                                   | Default        |
| ----------------------- | ----- | ------------------------------------------------------------- | -------------- |
| `--file <file>`         | `-f`  | HTML file(s) to search (repeatable)                           | settings paths |
| `--tag <tag>`           | `-t`  | Filter by tag name (e.g. `--tag a`)                           | —              |
| `--attr <name[=value]>` | `-a`  | Filter by attribute presence or value (repeatable)            | —              |
| `--text <pattern>`      |       | Case-insensitive substring match on element text content      | —              |
| `--json <fields>`       |       | Output JSON with specified fields: `tag,attrs,text,html,path` | —              |
| `--jq <expr>`           | `-q`  | Filter JSON output with a jq expression                       | —              |
| `--limit <n>`           | `-L`  | Maximum results                                               | `30`           |

When `--file` is omitted, search expands the catalog directory registered in the resolved config file. Errors if no catalog directory is configured and the default `~/.ctlgr-cli/catalog/` is empty.

**Selector** is a CSS selector (tag, class, id, attribute, combinators). When `--tag` is given without a selector, it becomes the selector. With neither, matches all elements (`*`).

**Attribute filters** (`--attr`) narrow results after selector matching:

- `--attr href` — element must have an `href` attribute
- `--attr class=nav` — element must have `class` exactly equal to `nav`

Multiple `--attr` flags are ANDed together.

**Examples**

```sh
# All links in a specific file
ctlgr search "a" --file page.html

# Search all registered catalog files
ctlgr search "a"

# Nav links, JSON output
ctlgr search "nav a" --json tag,attrs,text --file page.html

# All headings across multiple files
ctlgr search "h1,h2,h3" -f index.html -f about.html --json tag,text

# Limit results
ctlgr search "p" --file page.html -L 5

# jq post-filter
ctlgr search "a" --json tag,attrs --file page.html -q '.[] | .attrs.href'
```

**Plain output format** (no `--json`):

```
<tag attr="value" ...> text content
```

**JSON output** returns an array of objects with only the requested fields present.

---

### update

Check for a newer version and upgrade if one is available.

```
ctlgr update
```

Queries crates.io for the latest published version. If newer than the running
binary, runs `cargo install ctlgr` to upgrade in-place. Exits non-zero if the
version fetch or install fails.

A background check also runs automatically on each command invocation (at most
once every 24 hours) and prints a notice to stderr when a new version is
available:

```
notice: ctlgr 0.2.0 is available (you have 0.1.0). Run `ctlgr update` to upgrade.
```

---

### config

Manage config files and registered search paths.

```
ctlgr config <subcommand>
```

| Subcommand            | Description                                                        |
| --------------------- | ------------------------------------------------------------------ |
| `init <path>`         | Create `.ctlgr` with the given catalog directory                   |
| `init --local <path>` | Create `.ctlgr.local` instead (gitignored, higher priority)        |
| `list`                | Show the resolved catalog path (or the default if none configured) |

To change the path, delete `.ctlgr` and re-run `config init`. To revert to the default, delete `.ctlgr`.

**Config file priority** (first found wins, walking up from CWD):

1. `.ctlgr.local` — personal overrides, not committed
2. `.ctlgr` — per-directory config
3. `~/.ctlgr-cli/settings.json` — global fallback
4. `~/.ctlgr-cli/catalog/` — hardcoded default (used when no path is configured)

**Examples**

```sh
# Create a config for this directory
ctlgr config init ~/catalog

# Create a local-only override
ctlgr config init --local ~/my-personal-catalog

# Show the resolved path
ctlgr config list
```

**Config file schema** (`.ctlgr`):

```json
{
  "path": "/Users/you/catalog",
  "excluded": ["AGENTS\\.md", "drafts/"]
}
```

- `path` — catalog directory; all `*.html` and `*.md` files under it are searched recursively.
- `excluded` — array of regex patterns; matching files are omitted from search and lint. Patterns are matched against the full file path. Invalid patterns are silently skipped. **Merged across all config levels** — patterns from `.ctlgr`, ancestor `.ctlgr` files, and `~/.ctlgr-cli/settings.json` are all applied together.

---

## Development

### Dev tools

Declared in `[package.metadata.tools]` — install before contributing:

```sh
cargo install committed   # enforces Conventional Commits via commit-msg hook
cargo install dprint      # formats .md files via pre-commit hook
```

### Test layout

Tests mirror `src/`. Each module has a corresponding file in `tests/` that
exercises its public API. `tests/integration.rs` is the exception — it spawns
the compiled binary and tests CLI behaviour end-to-end.

| File                   | Mirrors           | Covers                                         |
| ---------------------- | ----------------- | ---------------------------------------------- |
| `tests/settings.rs`    | `src/settings.rs` | Config discovery, load/save, path expansion    |
| `tests/search.rs`      | `src/search.rs`   | Selector building, filtering, matching, output |
| `tests/integration.rs` | (exception)       | Full CLI via binary — all commands and flags   |

New source modules must have a matching `tests/<module>.rs`. Inline
`#[cfg(test)]` blocks are reserved for cases where private access is
unavoidable and extraction to a separate file would require exposing
implementation details that should stay hidden.

---

## Notes for agents

- Selectors follow CSS3: tag (`a`), class (`.nav`), id (`#main`), attribute (`[href]`), pseudo (`:first-child`), combinators (``, `>`, `+`, `~`).
- `--text` does a case-insensitive substring match against all descendant text nodes joined together. Combine with a specific selector to avoid matching every ancestor element.
- `--json` fields not in the requested list are omitted from output objects.
- `--file` may be repeated; results from all files are interleaved in document order, capped at `--limit` total.
- When `--file` is omitted, the catalog directory is searched recursively for `*.html` and `*.md` files at run time — newly added files are picked up automatically.
- `excluded` patterns from all config files in the ancestor chain are merged and applied together.
- Config resolution walks up from CWD; subdirectories automatically inherit the nearest ancestor config.
- Exit code `0` on success (including zero results); non-zero on parse or I/O errors.
