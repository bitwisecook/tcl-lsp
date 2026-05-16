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

### 6. Ship a custom builtin function

The same engine that registers `length`, `select`, `ip`, and the
other in-tree builtins is exposed as the public `@builtin`
decorator:

```python
from f5q import builtin

@builtin(
    "uppercase",
    summary="ASCII upper-case a string.",
    signatures=("uppercase(s: string) -> string",),
    examples=(".ltm.virtual[] | uppercase(.name)",),
    min_args=1,
    max_args=1,
)
def _uppercase(s):
    return str(s).upper()
```

After the plugin loads, the DSL calls it like any built-in:
`f5 q --raw 'uppercase(.ltm.virtual[].name)' bigip.conf`.  Set
`category="user"` (the default) to group plugin builtins together
in `--help-builtins`.  The advanced flags
(`special_form=True`, `with_ctx=True`, `stream_aware=True`) are
documented in the [renderer contract design
doc](../design/f5-query-renderer-contract.md) and modelled by the
in-tree builtins.

### 7. Ship a custom input format

```python
from f5q import input_format

@input_format(
    "yaml",
    summary="YAML side-input (single document or stream).",
)
def _parse_yaml(source, *, uri, options=()):
    import yaml  # third-party — install separately
    loaded = list(yaml.safe_load_all(source))
    return loaded[0] if len(loaded) == 1 else loaded
```

After the plugin loads, the CLI accepts
`--input yaml routes=routes.yaml` and the Python API accepts
`input_specs={"routes.yaml": InputSpec(kind="yaml")}`.  `--input`
is the generic flag the engine routes through any registered
format — the typed `--input-json` / `--input-jsonl` / `--input-csv`
/ `--input-f5log` shorthands are kept for compatibility.

### 8. Auto-load plugins from `~/.config/f5q/plugins/`

The engine scans `$XDG_CONFIG_HOME/f5q/plugins/*.py` (default
`~/.config/f5q/plugins/*.py`) on the first registry access.  Drop a
file in that directory and it loads transparently the next time
`f5 q` runs or any script imports a registry helper — no
`import my_plugin` ceremony, no PYTHONPATH dance:

```sh
mkdir -p ~/.config/f5q/plugins
cat > ~/.config/f5q/plugins/my_extensions.py <<'PY'
from f5q import builtin, renderer, input_format

@builtin("uppercase", summary="upper", min_args=1, max_args=1)
def _u(s):
    return str(s).upper()

@renderer("count-only", summary="row count", accepts="any")
def _c(values, **opts):
    return f"{len(values)} rows\n"

@input_format("simple-list", summary="newline list")
def _l(source, *, uri, options=()):
    return [line for line in source.splitlines() if line.strip()]
PY

f5 q --help-plugins     # confirm the file was picked up
f5 q --help-builtins    # 'uppercase' appears under category 'user'
f5 q --help-renderers   # 'count-only' appears alongside the built-ins
f5 q --help-inputs      # 'simple-list' appears alongside json/jsonl/csv/f5log
```

Files starting with `_` are skipped (helper modules a plugin
imports privately).  Sub-folders are scanned too, so a multi-file
plugin can structure itself however it likes.  A plugin file that
fails to import (syntax error, missing dependency, bad decorator
argument) prints a warning to stderr and is skipped — one broken
plugin can't kill the rest.  Use `f5 q --help-plugins` to see
which files actually loaded.

The same loader runs from Python:

```python
import f5q
f5q.load_user_plugins()        # idempotent — call once at startup
print(f5q.xdg_plugin_dir())    # diagnostic: where the loader looks
print(f5q.list_renderers())    # everything available after loading
```

`load_user_plugins(force=True)` re-scans (test fixtures use this
to inject plugins per-test).

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
