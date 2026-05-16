# KCS: How do I use the `f5 query` engine from a Python script?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

I want to drive the same jq-flavoured BIG-IP query engine `f5 query`
uses from inside my own Python script — to feed a custom report, plot
the results, or run the same query against many configs in a loop —
without shelling out to the CLI and parsing stdout.

## Before you start

- `tcl-lsp` installed in the same Python environment as your script
  (`uv pip install tcl-lsp` for releases; an editable checkout works
  too).
- A `bigip.conf` / SCF file the query will run against.
- Familiarity with the DSL — run `f5 query --help-dsl` once or skim
  the [reference manual](../references/f5_query/manual.md).

## Answer

The engine is exposed as the top-level **`f5q`** package.  Everything
external scripts normally need lives at the top level — no reaching
into `core.bigip.query` internals.

### 1. Run a query and iterate results

```python
from f5q import Query

run = Query(".ltm.virtual[] | .name").run(paths=["bigip.conf"])

for name in run:
    print(name)
```

`Query.run` accepts either `paths=[...]` (files read from disk) or
`sources={uri: text}` (already loaded), or both — when both name the
same URI the in-memory text wins.

### 2. Get typed values back

The result wraps the underlying `QueryResult` and exposes the shapes
external scripts normally want:

```python
from f5q import Query

run = Query(".ltm.virtual[]").run(paths=["bigip.conf"])

run.values()    # flat list of every value the query produced
run.first()     # the first value, or None
run.objects()   # only the ObjectRef instances
run.paths()     # full_path of every ObjectRef / PathRef
run.rows()      # [QueryRow(uri, value), ...] keeping the source URI
```

`ObjectRef`, `PathRef`, and `Stream` are re-exported from `f5q` so
your script can pattern-match on them without importing anything from
`core.bigip.query`.

### 3. Apply a mutation and read back the rewritten config

```python
from f5q import Query

run = Query('.ltm.virtual[].destination |= ip("192.168.9.0/24", .)').run(
    paths=["bigip.conf"],
)

if run.result.has_mutation:
    rewritten = run.edits("bigip.conf")
    if rewritten is not None:
        print(rewritten)  # the new file contents
```

Edits stay in memory — `Query.run` never writes to disk.  The CLI's
`--in-place` is implemented on top of the same API: it just calls
`Path(uri).write_text(run.edits(uri))`.

### 4. Render with a built-in plugin

The same renderer plugins `f5 q --render NAME` dispatches to are
reachable from Python:

```python
from f5q import Query

# ASCII Gantt of pool-member up/down transitions.
print(
    Query("""
        f5log_load("ltm.log")[]
        | select(.module == "01340011" or .module == "01340012")
        | tsv(.timestamp,
              (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
              (if .module == "01340011" then "DOWN" else "UP" end))
    """)
    .run(paths=["bigip.conf"])
    .render("gantt", **{"unit-minutes": "10"})
)
```

Three renderers ship in-tree: `gantt`, `ascii-blocks`, `mermaid`.
List them with `from f5q import list_renderers; print(list_renderers())`
or via `f5 q --help-renderers` on the CLI.

### 5. Ship a custom renderer

```python
from f5q import renderer

@renderer(
    "md-table",
    summary="Markdown table of results.",
    accepts="list of dicts",
)
def _render_md_table(values, **opts):
    if not values:
        return "(no rows)\n"
    headers = list(values[0].keys())
    out = ["| " + " | ".join(headers) + " |",
           "| " + " | ".join("---" for _ in headers) + " |"]
    for row in values:
        out.append("| " + " | ".join(str(row.get(h, "")) for h in headers) + " |")
    return "\n".join(out) + "\n"
```

After importing the module that defines `@renderer`, the CLI picks it
up automatically: `f5 q --render md-table ...`.  Plugin discovery is
in-tree only today — a third-party package needs to be imported
before `f5 q` runs (e.g. via `python -c 'import my_pkg; ...'` or by
shipping a tiny wrapper script).

## How to tell it worked

`uv run python -c "from f5q import Query; print(len(Query('.ltm.virtual[]').run(paths=['bigip.conf']).values()))"`
prints the virtual-server count of the file.  `f5 q --help-renderers`
lists every renderer your script registered.

## Related

- [KCS index](README.md)
- [KCS: feature — `f5 query` renderers](features/kcs-feature-f5-query-renderers.md)
- [KCS: how-to — reproduce an HTTP monitor with `f5 query`](kcs-howto-reproduce-http-monitor-with-query.md)
- [`f5 query` reference manual](../references/f5_query/manual.md)
- [Design — `f5 query` renderer contract](../design/f5-query-renderer-contract.md)
