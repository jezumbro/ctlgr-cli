# Data Model: Add `ctlgr convert` Command

## Entities

### ConvertArgs

CLI arguments for the `ctlgr convert` subcommand.

| Field   | Type             | Description                                                                     |
| ------- | ---------------- | ------------------------------------------------------------------------------- |
| file    | `Vec<String>`    | Explicit `.md` file paths (repeatable; mutually exclusive with `--dir`)         |
| dir     | `Option<String>` | Directory to walk for `*.md` files (defaults to config catalog if both omitted) |
| dry_run | `bool`           | Print status lines without writing or deleting any files                        |

### ConvertOutcome (internal, per file)

Result of processing one source file.

| Variant   | Description                                                          |
| --------- | -------------------------------------------------------------------- |
| Converted | New `.html` file created; source `.md` removed                       |
| Merged    | Rendered Markdown merged into existing `.html`; source `.md` removed |
| Skipped   | `--dry-run` mode; no files touched                                   |

## State Transitions

```
source .md file
  │
  ├─ target .html does NOT exist → Converted
  │     write new self-contained .html
  │     remove .md
  │
  └─ target .html exists → Merged
        insert rendered body before </body>
        remove .md
```

In `--dry-run` mode, the determination of Converted vs Merged is still made (to print the correct
status line), but no files are written or deleted.

## Path Derivation

Target path = source path with `.md` suffix replaced by `.html`.
If source has no `.md` suffix, `.html` is appended.

Examples:

- `docs/readme.md` → `docs/readme.html`
- `catalog/page.md` → `catalog/page.html`

## File Discovery

| Source           | Behavior                                                   |
| ---------------- | ---------------------------------------------------------- |
| `--file <paths>` | Use exactly the listed paths; no directory walk            |
| `--dir <path>`   | Glob `<path>/**/*.md` recursively                          |
| (neither)        | Use `settings::expand_path` → configured catalog directory |
