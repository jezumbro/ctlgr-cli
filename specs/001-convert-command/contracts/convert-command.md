# CLI Contract: `ctlgr convert`

## Synopsis

```
ctlgr convert [--file <path>]... [--dir <path>] [--dry-run]
```

## Flags

| Flag            | Short | Type   | Default                  | Description                               |
| --------------- | ----- | ------ | ------------------------ | ----------------------------------------- |
| `--file <path>` | `-f`  | String | (empty)                  | `.md` file to convert (repeatable)        |
| `--dir <path>`  |       | String | (configured catalog dir) | Directory to walk for `*.md` files        |
| `--dry-run`     |       | bool   | false                    | Preview without writing or deleting files |

When both `--file` and `--dir` are omitted, the configured catalog directory is used (same
resolution as `ctlgr search`).

## Output (stdout)

One line per file, format:

```
<src>: converted to <dst>
<src>: merged into <dst>
```

In `--dry-run` mode, the same lines are printed; no files are changed.

## Exit Codes

| Code | Meaning                                             |
| ---- | --------------------------------------------------- |
| 0    | All files processed successfully (or 0 files found) |
| 1    | One or more files failed (I/O or parse error)       |

## Errors (stderr)

All error messages go to stderr. A single file failure is reported and the process exits 1.

## Changed: `ctlgr lint`

The `prefer-html` rule is removed. `ctlgr lint` no longer accepts `.md` files in its processing
pipeline. The `--write` flag only fixes `no-style-blocks` and `no-inline-styles`.

Updated `after_help` for `ctlgr lint`:

```
MODES:
  check (default)  report violations, exit non-zero if any found
  --write          fix in place: strip style blocks and inline styles

RULES (all enabled by default; configure via lint.rules in settings):
  no-style-blocks   <style> blocks are not allowed
  no-inline-styles  style="..." attributes are not allowed
```
