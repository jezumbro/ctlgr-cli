Add or update documentation in `ctlgr-docs/`.

## Step 1 — Discover the existing doc landscape

Before writing anything, use `ctlgr` to understand what already exists.

```sh
# Full outline: all headings across all docs
ctlgr search "h1,h2,h3" --json tag,text,path

# Find content related to the topic you're documenting
ctlgr search --text "<topic keyword>" --json tag,text,path

# Find existing command docs
ctlgr search "[data-command]" --json attrs,text

# Find existing sections by id
ctlgr search "[id]" --json tag,attrs,text,path
```

Use these results to decide:

- Does the topic already exist? If so, **extend the existing file** rather than creating a new one.
- Which file is the closest home for new content?
- What sections and ids are already taken?

## Step 2 — Determine scope

- **New file**: topic is large enough to deserve its own page, or no existing file covers it
- **New section in existing file**: topic is a sub-topic of something already documented
- **Update existing section**: content is stale or incomplete

## Step 3 — Write the content

Follow the catalog format:

- Valid HTML5 with `<!DOCTYPE html>`, `<head>`, `<body>`
- `<title>` describes the page topic
- Semantic structure: `<section id="...">`, `<article data-type="...">`, `<h1>`–`<h4>`
- Data attributes for machine-readable metadata: `data-type`, `data-command`, `data-topic`, etc.
- `<dl>/<dt>/<dd>` for term/definition pairs
- `<pre><code>` for all code and shell examples
- `<a href="...">` links to related docs or external resources

### Structural template (new file)

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>ctlgr — Topic Name</title>
</head>
<body>
  <h1>Topic Name</h1>
  <p>One-line description.</p>

  <section id="overview">
    <h2>Overview</h2>
    <p>...</p>
  </section>

  <section id="examples">
    <h2>Examples</h2>
    <article data-type="example" data-topic="<topic>">
      <h3>Example title</h3>
      <pre><code>ctlgr search ...</code></pre>
    </article>
  </section>
</body>
</html>
```

## Step 4 — Verify searchability

After writing, confirm the content is reachable with ctlgr:

```sh
# Check headings are indexed
ctlgr search "h1,h2,h3" --file ctlgr-docs/<file>.html --json tag,text

# Check key terms are findable
ctlgr search --text "<key term>" --file ctlgr-docs/<file>.html --json tag,text,path

# Check data attributes are queryable
ctlgr search "[data-command]" --file ctlgr-docs/<file>.html --json attrs,text
```

## Step 5 — Update index.html if needed

If the new doc adds a top-level topic, link it from `index.html` under the relevant section.

## Conventions

| Element                         | Use for                                          |
| ------------------------------- | ------------------------------------------------ |
| `<section id="...">`            | Major logical groupings within a page            |
| `<article data-type="command">` | Individual commands or subcommands               |
| `<article data-type="example">` | Usage examples                                   |
| `<article data-type="flag">`    | Individual CLI flags                             |
| `<dl>`                          | Term/definition pairs (glossaries, option lists) |
| `data-topic="<name>"`           | Tag content for topic-based filtering            |
| `data-since="<version>"`        | Mark when a feature was added                    |
| `data-status="stable\|beta"`    | Stability indicator                              |

## Anti-patterns to avoid

- Creating a new file for content that fits in an existing section — check with `ctlgr search` first
- Generic `<div>` and `<span>` without attributes (nothing to query)
- Missing `id` attributes on sections (makes selector targeting harder)
- Prose-heavy paragraphs without structural markup
- Duplicate content — cross-link instead: `<a href="config.html#init">`
