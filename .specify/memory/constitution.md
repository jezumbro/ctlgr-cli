<!--
SYNC IMPACT REPORT
==================
Version change: (none) → 1.0.0 (initial ratification)
Modified principles: N/A — first fill of template placeholders
Added sections: Core Principles (I–V), Technology Stack, Development Workflow, Governance
Removed sections: none
Templates updated:
  ✅ .specify/templates/plan-template.md — Constitution Check gates aligned
  ✅ .specify/templates/spec-template.md — scope constraints aligned
  ✅ .specify/templates/tasks-template.md — task types aligned
Deferred: none
-->

# ctlgr Constitution

## Core Principles

### I. Command Clarity

Each subcommand MUST do exactly one thing.
`search` searches; `lint` lints style violations; `convert` converts `.md` → `.html`; `config`
manages configuration. A command MUST NOT perform destructive file transformation as a side
effect of a non-destructive operation (e.g., `lint` MUST NOT write or delete `.md` files).
New subcommands require a clear, single-sentence purpose statement before implementation begins.

### II. Test-First (NON-NEGOTIABLE)

Every new source module MUST have a matching `tests/<module>.rs` integration test file.
Inline `#[cfg(test)]` blocks are reserved only for cases where private access is unavoidable and
extraction would require exposing implementation details that MUST remain hidden.
Tests MUST be written and confirmed failing before implementation is committed.

### III. Agent-Friendly I/O

All commands MUST support machine-readable output alongside human-readable output.
`--json <fields>` selects structured output; human-readable is the default.
Commands MUST exit `0` on success (including zero results) and non-zero on any parse or I/O error.
Error messages MUST go to stderr; all result output MUST go to stdout.

### IV. Config-Driven Discovery

File discovery MUST be driven by config, not hardcoded paths.
Config resolution MUST walk up from CWD; subdirectory configs inherit ancestor settings.
`excluded` patterns from all config files in the ancestor chain MUST be merged and applied together.
Files passed via `--file` are never filtered by `excluded`.

### V. Simplicity

Do not add features, abstractions, or error handling beyond what the task requires.
Three similar lines of code are preferable to a premature abstraction.
No half-finished implementations. No backwards-compatibility shims for unused code paths.
Complexity introduced beyond these principles MUST be justified in the PR description.

## Technology Stack

- **Language**: Rust (edition 2024)
- **CLI framework**: clap (derive feature)
- **HTML parsing**: scraper (CSS3 selectors)
- **Markdown → HTML**: pulldown-cmark
- **Config/serialization**: serde + serde_json
- **Testing**: `assert_cmd`, `predicates`, `tempfile` (integration tests in `tests/`)
- **Benchmarking**: criterion

All dependencies MUST be added via `cargo add`; version pins in `Cargo.toml` use
`major.minor` ranges (e.g., `"4"`, `"3"`) unless a specific patch is required for a known fix.

## Development Workflow

- New features MUST be developed on a feature branch; PRs target `main`.
- CI MUST pass (build + tests + benchmark correctness check) before merge.
- Commit messages MUST follow Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, etc.).
- Subject line ≤ 50 characters; body included only when the "why" is non-obvious from the diff.
- The `prefer-html` lint rule (and any rule that triggers file transformation) is deprecated in
  favor of the dedicated `ctlgr convert` command.

## Governance

This constitution supersedes all other informal practices for this repository.
Amendments require: (1) a PR updating this file, (2) a version bump per semver rules below,
and (3) propagation to dependent templates in `.specify/templates/`.

**Versioning policy**:

- MAJOR: backward-incompatible governance or principle removals/redefinitions
- MINOR: new principle or section added, or materially expanded guidance
- PATCH: clarifications, wording, typo fixes

All PRs MUST verify compliance with the Core Principles before merge.
Use `CLAUDE.md` for runtime agent guidance; this constitution governs design decisions.

**Version**: 1.0.0 | **Ratified**: 2026-05-30 | **Last Amended**: 2026-05-30
