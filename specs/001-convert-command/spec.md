# Feature Specification: Add `ctlgr convert` Command

**Feature Branch**: `worktree-feat+convert-command`
**Created**: 2026-05-30
**Status**: Draft
**Input**: User description: "Add a dedicated `ctlgr convert` subcommand that converts a directory
of `.md` files to `.html` catalog files. Simultaneously, strip that responsibility from
`ctlgr lint` so lint is a pure linting tool."

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Convert Markdown Files to HTML Catalog (Priority: P1)

A developer has accumulated `.md` files in their catalog directory and wants to convert them all to
`.html` catalog format so they can be searched with `ctlgr search`. They run `ctlgr convert` and
each `.md` is converted to a self-contained `.html` file; the source `.md` is removed on success.

**Why this priority**: This is the core value of the command — without it, the command has no
purpose. All other stories depend on or extend this flow.

**Independent Test**: Run `ctlgr convert --dir <dir-with-md-files>` against a directory containing
one `.md` file. Verify the `.html` file appears, the `.md` is removed, and one status line is
printed.

**Acceptance Scenarios**:

1. **Given** a directory with one `.md` file and no existing `.html` at the target path,
   **When** `ctlgr convert --dir <dir>` is run,
   **Then** a self-contained `.html` catalog file appears at the same path with `.html` extension,
   the source `.md` is removed, and output is `<src>: converted to <dst>`.

2. **Given** a directory with multiple `.md` files,
   **When** `ctlgr convert --dir <dir>` is run,
   **Then** each is converted and removed in turn, with one status line per file.

3. **Given** a single `.md` file path passed via `--file`,
   **When** `ctlgr convert --file path/to/file.md` is run,
   **Then** only that file is converted; other `.md` files in the same directory are untouched.

---

### User Story 2 - Merge Into Existing HTML Catalog (Priority: P2)

A developer runs `ctlgr convert` in a directory that already has an `.html` file at the target
path. Rather than overwriting, the command merges the rendered Markdown content into the existing
`.html` document before `</body>`.

**Why this priority**: Catalog files may have hand-edited metadata or additional content that must
not be discarded. Safe merge is required for incremental workflow.

**Independent Test**: Create a `.md` file whose target `.html` already exists with content. Run
`ctlgr convert`. Verify the `.html` now contains both the original content and the newly rendered
Markdown, and the status line reads `<src>: merged into <dst>`.

**Acceptance Scenarios**:

1. **Given** an `.md` file and a pre-existing `.html` at the target path,
   **When** `ctlgr convert` is run,
   **Then** the rendered Markdown body is inserted before `</body>` in the existing `.html`,
   the source `.md` is removed, and output is `<src>: merged into <dst>`.

---

### User Story 3 - Dry-Run Preview (Priority: P3)

A developer wants to see what `ctlgr convert` would do before committing to the changes. They pass
`--dry-run` and get a report of what would be converted or merged, without any files being created,
modified, or deleted.

**Why this priority**: Safe by default. Important for CI pipelines and cautious users, but the core
conversion still works without it.

**Independent Test**: Run `ctlgr convert --dry-run --dir <dir>` and verify no files are changed
but the expected status lines are still printed.

**Acceptance Scenarios**:

1. **Given** a directory with `.md` files,
   **When** `ctlgr convert --dry-run --dir <dir>` is run,
   **Then** status lines are printed for each file as if conversion would happen, but no `.html`
   files are created or modified and no `.md` files are removed.

---

### User Story 4 - Lint Is Pure Linting (Priority: P1)

After the feature ships, `ctlgr lint` only reports or fixes HTML style violations. It does not
convert `.md` files to `.html` and does not offer a `prefer-html` rule.

**Why this priority**: This is the other half of the separation-of-concerns goal. Lint must not
have side effects beyond HTML style fixes. Tied in priority with the convert command itself.

**Independent Test**: Run `ctlgr lint --write` against a directory containing `.md` files. Verify
no `.md` files are converted and no `.html` files are created.

**Acceptance Scenarios**:

1. **Given** a directory with both `.md` files and `.html` files with style violations,
   **When** `ctlgr lint --write` is run,
   **Then** style violations in `.html` files are fixed, `.md` files are untouched, and no new
   files are created.

2. **Given** a `ctlgr lint` invocation,
   **When** the command runs,
   **Then** the `prefer-html` rule is not reported and does not appear in help text.

---

### Edge Cases

- What happens when a `.md` file has no meaningful content (empty body)?
  Convert still succeeds; produces a minimal `.html` with an empty body.
- What happens when the target `.html` is malformed (missing `</body>`)?
  Command exits non-zero with an error message on stderr identifying the file.
- What happens when a file passed via `--file` does not exist?
  Command exits non-zero with an error on stderr; no other files are affected.
- What happens when `--dir` and `--file` are both omitted?
  The configured catalog directory is used (same discovery as `ctlgr search`).
- What happens when conversion of one file fails mid-batch?
  The command reports the error and exits non-zero, leaving already-converted files in place.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: `ctlgr convert` MUST walk a specified directory for `*.md` files when `--dir` is
  provided, or use the configured catalog directory when neither `--dir` nor `--file` is given.
- **FR-002**: `ctlgr convert` MUST accept one or more explicit `--file <path>` arguments and
  convert only those files.
- **FR-003**: `ctlgr convert` MUST produce a self-contained `.html` catalog document at the target
  path (same directory, `.html` extension replacing `.md`) for each source file.
- **FR-004**: `ctlgr convert` MUST merge the rendered Markdown body before `</body>` when an
  `.html` file already exists at the target path, rather than overwriting the entire file.
- **FR-005**: `ctlgr convert` MUST remove the source `.md` file after successful conversion.
- **FR-006**: `ctlgr convert` MUST print exactly one status line per file to stdout:
  `<src>: converted to <dst>` or `<src>: merged into <dst>`.
- **FR-007**: `ctlgr convert --dry-run` MUST print status lines without creating, modifying, or
  removing any files.
- **FR-008**: `ctlgr convert` MUST exit non-zero on any I/O or parse error.
- **FR-009**: `ctlgr lint` MUST NOT convert `.md` files to `.html` or remove any `.md` files.
- **FR-010**: The `prefer-html` lint rule MUST be removed from `ctlgr lint` entirely (no report,
  no write, no help text mention).
- **FR-011**: `ctlgr lint --write` MUST only fix `no-style-blocks` and `no-inline-styles`
  violations in `.html` files.

### Key Entities

- **Source file**: A `.md` file discovered by directory walk or passed via `--file`.
- **Target file**: The `.html` file at the same path as the source but with `.html` extension.
- **Conversion result**: One of "converted" (new file created) or "merged" (content merged into
  existing target).

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: All `.md` files in a catalog directory can be converted to `.html` with a single
  command invocation, with one status line per file confirming the outcome.
- **SC-002**: After running `ctlgr convert`, running `ctlgr lint` reports zero violations related
  to unconverted Markdown files.
- **SC-003**: Running `ctlgr lint --write` against a directory containing `.md` files leaves those
  files untouched 100% of the time.
- **SC-004**: `--dry-run` mode produces identical output to a real run but results in zero file
  system changes.
- **SC-005**: `ctlgr lint` help text contains no mention of `prefer-html` or `.md` conversion.

## Assumptions

- The target `.html` path is always derived by replacing the `.md` extension with `.html` in the
  same directory as the source; no path remapping is needed.
- The existing conversion logic in `src/lint.rs` (`convert_md_to_html`, `md_to_html`,
  `md_to_html_fragment`, `merge_html`) is correct and can be moved to `src/convert.rs` without
  behavioral changes.
- `ctlgr convert` does not register converted files in the catalog config automatically; that
  remains a responsibility of `ctlgr config` if needed.
- No JSON output flag is required for `ctlgr convert` in this iteration (human-readable only).
- The `--dir` flag defaults to the configured catalog directory (same resolution as `ctlgr search`)
  when omitted alongside `--file`.
