# Tasks: Add `ctlgr convert` Command

**Input**: Design documents from `specs/001-convert-command/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup

**Purpose**: Expose `convert` module and wire it into the CLI.

- [x] T001 Add `pub mod convert;` to `src/lib.rs`
- [x] T002 Add `Convert(convert::ConvertArgs)` variant to `Commands` enum in `src/main.rs`, add `use ctlgr::convert;`, and add match arm `Commands::Convert(args) => convert::run(&args)`

---

## Phase 2: Foundational — Create `src/convert.rs` Skeleton

**Purpose**: Define the public API before tests can be written against it. Must be complete (even if functions are stubs) before test tasks can compile.

- [x] T003 Create `src/convert.rs` with:
  - `ConvertArgs` struct (clap derive): fields `file: Vec<String>` (`-f/--file`), `dir: Option<String>` (`--dir`), `dry_run: bool` (`--dry-run`)
  - Stub `pub fn run(args: &ConvertArgs) -> anyhow::Result<()>` returning `Ok(())`
  - Move `md_html_path`, `md_to_html`, `md_to_html_fragment`, `merge_html` from `src/lint.rs` verbatim
  - Add stub `pub fn convert_md_to_html(md_path: &str, dry_run: bool) -> anyhow::Result<()>` returning `Ok(())`

**Checkpoint**: `cargo build` passes (stubs compile). Tests will be written next.

---

## Phase 3: User Story 1 — Convert Markdown Files to HTML Catalog (P1) 🎯 MVP

**Goal**: `ctlgr convert --file <path>` and `ctlgr convert --dir <dir>` convert `.md` to `.html`
and remove the source file.

**Independent Test**: Run `ctlgr convert --file <tmpdir>/page.md`; verify `page.html` exists and
`page.md` is removed.

### Tests for User Story 1

> Write these tests FIRST; confirm they FAIL before implementing T009.

- [x] T004 [P] [US1] Write conversion tests in `tests/convert.rs`:
  - `convert_md_to_html_creates_html_file` — write `.md`, call `convert_md_to_html(path, false)`, assert `.html` exists with `<title>`
  - `convert_md_to_html_removes_md_file` — write `.md`, call `convert_md_to_html(path, false)`, assert `.md` gone
  - `convert_md_to_html_html_has_no_style_violations` — assert generated HTML passes `check_html`
  - `convert_via_file_flag` — integration test: `ctlgr convert --file <path>` succeeds, `.html` created, `.md` removed
  - `convert_via_dir_flag` — integration test: `ctlgr convert --dir <dir>` converts all `.md` in dir

- [x] T005 [P] [US1] Move shared helper tests from `tests/lint.rs` to `tests/convert.rs` (update imports to `ctlgr::convert::*`):
  - `md_to_html_produces_html_document`
  - `md_to_html_uses_first_h1_as_title`
  - `md_to_html_falls_back_to_document_title_when_no_h1`
  - `md_to_html_renders_markdown_content`
  - `md_to_html_generates_no_style_violations`
  - `md_html_path_replaces_md_extension`
  - `md_html_path_appends_html_when_no_md_suffix`
  - `merge_html_inserts_before_closing_body`
  - `merge_html_appends_when_no_body_tag`
  - `merge_html_uses_last_body_tag`
  - `md_to_html_fragment_renders_without_document_wrapper`

### Implementation for User Story 1

- [x] T006 [US1] Implement `convert_md_to_html(md_path, dry_run)` in `src/convert.rs`:
  - Read source `.md`
  - Derive `html_path` via `md_html_path`
  - If `html_path` exists: compute merged HTML; if `!dry_run` write and remove `.md`; print `"<src>: merged into <dst>"`
  - Else: compute full HTML; if `!dry_run` write and remove `.md`; print `"<src>: converted to <dst>"`

- [x] T007 [US1] Implement `run(args)` in `src/convert.rs`:
  - If `args.file` non-empty: use those paths
  - Else if `args.dir` is `Some(d)`: glob `<d>/**/*.md`
  - Else: call `settings::load()` + `settings::expand_path`, filter to `.md` files
  - For each path: call `convert_md_to_html(path, args.dry_run)?`; collect errors; exit non-zero if any

**Checkpoint**: `cargo test` passes for all US1 tests. `ctlgr convert --file <path>` works end-to-end.

---

## Phase 4: User Story 2 — Merge Into Existing HTML Catalog (P2)

**Goal**: When `.html` already exists at target path, rendered Markdown is merged before `</body>`.

**Independent Test**: Create `.md` + pre-existing `.html`; run `ctlgr convert --file <path>`; verify
`.html` contains both old and new content; `.md` removed; output says `merged into`.

### Tests for User Story 2

> Write these FIRST; confirm they FAIL before implementing.

- [x] T008 [P] [US2] Write merge tests in `tests/convert.rs`:
  - `convert_md_to_html_merges_when_html_already_exists` — old content preserved, new content added
  - `convert_md_to_html_merged_content_before_closing_body` — new content appears before `</body>`
  - `convert_prints_merged_into_when_html_exists` — assert stdout contains `"merged into"`

### Implementation for User Story 2

T006 already handles the merge case (it's part of `convert_md_to_html`). No additional
implementation required beyond what T006 delivers; only the tests above are new.

**Checkpoint**: All US2 tests pass. Existing US1 tests still pass.

---

## Phase 5: User Story 3 — Dry-Run Preview (P3)

**Goal**: `--dry-run` prints status lines without modifying any files.

**Independent Test**: Run `ctlgr convert --dry-run --dir <dir>`; verify status lines printed, no
files created/modified/deleted.

### Tests for User Story 3

- [x] T009 [P] [US3] Write dry-run tests in `tests/convert.rs`:
  - `dry_run_does_not_create_html` — after `convert_md_to_html(path, true)`, `.html` must not exist
  - `dry_run_does_not_remove_md` — after `convert_md_to_html(path, true)`, `.md` still exists
  - `dry_run_prints_would_convert` — stdout still contains `"converted to"` even in dry-run mode
  - `dry_run_via_cli_flag` — integration test: `ctlgr convert --dry-run --file <path>` exits 0,
    no file changes

### Implementation for User Story 3

The `dry_run` parameter in `convert_md_to_html` (T006) and the `args.dry_run` pass-through in
`run()` (T007) already implement this. No additional source changes required.

**Checkpoint**: All US3 tests pass. All prior tests still pass.

---

## Phase 6: User Story 4 — Lint Is Pure Linting (P1)

**Goal**: `ctlgr lint` no longer processes `.md` files or mentions `prefer-html`.

**Independent Test**: Run `ctlgr lint --write` against a dir with `.md` files; verify no `.md`
is converted and no `.html` is created.

### Tests for User Story 4

- [x] T010 [P] [US4] Write lint-purity tests in `tests/lint.rs`:
  - `lint_does_not_process_md_files` — integration test: `ctlgr lint --file page.md` succeeds with
    no output (`.md` files silently skipped, not converted)
  - `lint_write_does_not_convert_md` — `ctlgr lint --write --file page.md` exits 0, `.md` unchanged,
    no `.html` created
  - `lint_help_has_no_prefer_html` — `ctlgr lint --help` output does not contain `prefer-html`

### Implementation for User Story 4

- [x] T011 [US4] Strip conversion logic from `src/lint.rs`:
  - Delete the `if path.ends_with(".md")` branch in `run()` (lines 37–46)
  - Delete `convert_md_to_html`, `md_html_path`, `md_to_html`, `md_to_html_fragment`, `merge_html`
    (they now live in `src/convert.rs`)
  - Remove `use pulldown_cmark::...` import from `src/lint.rs`
  - Update `after_help` string in `LintArgs` to remove `prefer-html` bullet and the `.md → .html`
    note from the `--write` description

- [x] T012 [US4] Update `tests/lint.rs`:
  - Remove imports of moved functions: `convert_md_to_html`, `md_html_path`, `md_to_html`,
    `md_to_html_fragment`, `merge_html`
  - Confirm all remaining lint tests still pass (check_html, fix_html tests are unchanged)

**Checkpoint**: All US4 tests pass. `cargo test` green across all modules.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [x] T013 [P] Verify `cargo build --release` succeeds with no warnings
- [x] T014 [P] Run `cargo test` full suite; confirm zero failures
- [x] T015 Run `cargo clippy -- -D warnings` and fix any new lints introduced by this feature

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 (needs lib.rs updated so `convert` module exists)
- **Phase 3 (US1)**: Depends on Phase 2 (skeleton must compile for tests to import)
- **Phase 4 (US2)**: Tests can be written in parallel with Phase 3 implementation; implementation
  is delivered as part of T006
- **Phase 5 (US3)**: Same as Phase 4 — implementation already in T006/T007
- **Phase 6 (US4)**: Can start after Phase 2 (does not depend on convert tests passing); run after
  Phase 3 to avoid double-removing functions
- **Phase 7 (Polish)**: Depends on all story phases

### User Story Dependencies

- **US1** (P1): Foundation — everything else builds on this
- **US2** (P2): Merge logic is part of T006; only tests are separate
- **US3** (P3): Dry-run is part of T006/T007; only tests are separate
- **US4** (P1): Independent of US1–US3; can be worked in parallel once Phase 2 is done

### Parallel Opportunities Within Phases

- T004 and T005 (test authoring) can run in parallel
- T008 and T009 (test authoring) can run in parallel
- T010 (lint-purity test) can run in parallel with T008/T009
- T013, T014, T015 (polish) can run in parallel

---

## Parallel Example: Phase 3 (US1)

```
# Write tests in parallel:
Task T004: new conversion + CLI tests in tests/convert.rs
Task T005: migrate helper tests from tests/lint.rs to tests/convert.rs

# After T004 and T005 confirm failing:
Task T006: implement convert_md_to_html
Task T007: implement run()
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001, T002)
2. Complete Phase 2: Foundational skeleton (T003)
3. Write tests (T004, T005) — confirm failing
4. Implement (T006, T007)
5. **STOP and VALIDATE**: `ctlgr convert --file` and `--dir` work end-to-end

### Full Incremental Delivery

1. Setup + Foundational → builds cleanly
2. US1 → `ctlgr convert` works for new files
3. US2 → merge into existing `.html` works
4. US3 → `--dry-run` works
5. US4 → `ctlgr lint` is pure, no `.md` processing
6. Polish → clean build, tests, clippy

---

## Notes

- [P] tasks = can run in parallel (different files)
- [Story] label maps each task to its user story
- Tests MUST be written and confirmed failing before implementation
- `cargo build` must pass after Phase 2 before writing tests (skeleton must compile)
- Commit after each phase checkpoint
