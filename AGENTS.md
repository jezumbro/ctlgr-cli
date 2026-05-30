# ctlgr-cli — Agent Interface

`ctlgr` is a CLI for creating and searching `.html` catalog files. For commands, flags, config
schema, and development setup, see [README.md](README.md).

## Automatic skill triggers

**Adding or editing anything in `ctlgr-docs/`**: always follow `.claude/commands/add-doc.md` — run
the `ctlgr search` discovery step first before writing or modifying any file.

## Notes for agents

- Selectors follow CSS3: tag (`a`), class (`.nav`), id (`#main`), attribute (`[href]`), pseudo
  (`:first-child`), combinators (``, `>`, `+`, `~`).
- `--text` does a case-insensitive substring match against all descendant text nodes joined
  together. Combine with a specific selector to avoid matching every ancestor element.
- `--json` fields not in the requested list are omitted from output objects.
- `--file` may be repeated; results from all files are interleaved in document order, capped at
  `--limit` total.
- When `--file` is omitted, the catalog directory is searched recursively for `*.html` and `*.md`
  files at run time — newly added files are picked up automatically.
- `excluded` patterns from all config files in the ancestor chain are merged and applied together.
  Patterns are matched as regex against the full file path; directory exclusions work as substrings
  (e.g. `drafts/`).
- `excluded` only applies to files discovered via config — files passed via `--file` are never
  filtered.
- Config resolution walks up from CWD; subdirectories automatically inherit the nearest ancestor
  config.
- Exit code `0` on success (including zero results); non-zero on parse or I/O errors.

## Testing conventions

New source modules must have a matching `tests/<module>.rs`. Inline `#[cfg(test)]` blocks are
reserved for cases where private access is unavoidable and extraction to a separate file would
require exposing implementation details that should stay hidden.

<!-- SPECKIT START -->

For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan at
`../ctlgr-docs/specs/001-convert-command/plan.md`

<!-- SPECKIT END -->
