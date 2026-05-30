# Research: Add `ctlgr convert` Command

## Existing Conversion Logic

**Decision**: Move functions wholesale from `src/lint.rs` to `src/convert.rs`.
**Rationale**: The implementation is complete and correct. All five functions
(`convert_md_to_html`, `md_html_path`, `md_to_html`, `md_to_html_fragment`, `merge_html`)
live in `src/lint.rs` and are used only for `.md` → `.html` conversion. Moving them requires
no behavioral changes.
**Alternatives considered**: Refactoring conversion logic — rejected, no bugs and no new
requirements beyond `--dry-run` and `--dir` flag support.

## Dry-Run Support

**Decision**: Add a `dry_run: bool` parameter to `convert_md_to_html`.
**Rationale**: Rust has no default parameters. Adding the bool directly is the simplest approach.
When `dry_run` is true, the function prints the status line but skips all writes and deletes.
**Alternatives considered**: A separate `convert_md_to_html_dry_run` function — rejected, DRY
violation with no benefit.

## Directory Walk Strategy

**Decision**: Reuse the same `settings::expand_path` + glob pattern used by search/lint for
default discovery; for `--dir`, use `glob` to match `<dir>/**/*.md`.
**Rationale**: The project already depends on the `glob` crate. The existing `expand_path` helper
handles configured catalog discovery. No new dependency needed.
**Alternatives considered**: `walkdir` crate — rejected, unnecessary new dependency.

## `prefer-html` Rule Removal

**Decision**: Delete the `prefer-html` branch in `lint::run()` entirely, remove the
`"prefer-html"` string from the `after_help` text, and delete `convert_md_to_html` and
`md_html_path` from `lint.rs`.
**Rationale**: The issue spec explicitly removes this rule. Any partial removal (keeping rule
but no-op) would leave dead code and mislead users reading the help text.
**Alternatives considered**: Keeping `prefer-html` as check-only (no write) pointing users to
`ctlgr convert` — described as an open question in the issue; decided against to keep scope clear.

## Test Migration

**Decision**: Move all conversion-related tests from `tests/lint.rs` to `tests/convert.rs`.
Update imports from `ctlgr::lint::*` to `ctlgr::convert::*`. Update `convert_md_to_html`
call sites to pass `dry_run: false`.
**Rationale**: Tests MUST live in the module they test per the project constitution.
**Alternatives considered**: Leave tests in `tests/lint.rs` with cross-module imports — rejected,
violates constitution Principle II.

## Module Exposure

**Decision**: Add `pub mod convert;` to `src/lib.rs`.
**Rationale**: Required for integration tests in `tests/convert.rs` to import via `ctlgr::convert`.
