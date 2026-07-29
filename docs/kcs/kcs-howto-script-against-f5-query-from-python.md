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

The engine is exposed via the **`dialects.f5.query`** package.  Everything
external scripts normally need lives at the top level — import it once
under a short alias and all the examples below work unchanged.

### 1. The one-liner — `f5q.q()`

`f5q.q()` is the polymorphic single-call entry.  The first string
that is **not** an existing file path is the expression; everything
else is an input (file path, `Sources`, a prior result, or in-memory
data):

```python
from dialects.f5 import query as f5q

# Expression + file.
for name in f5q.q(".ltm.virtual[] | .name", "bigip.conf"):
    print(name)
```

The return value is a `QueryRun` — it iterates like a list, indexes
like a list, supports `len()` / `bool()`, and exposes one method per
"what shape do I want this in" question.

### 2. Pre-staging — `f5q.load()`

When one corpus feeds many queries, load it once:

```python
from dialects.f5 import query as f5q

corpus = f5q.load("ltm.conf", "gtm.conf")
virtuals = f5q.q(".ltm.virtual[]", corpus)
pools = f5q.q(".ltm.pool[]", corpus)
```

`load()` accepts file paths, `pathlib.Path` objects, other
`Sources` (merged in), and **in-memory** dicts / lists / tuples
(wrapped as JSON data the DSL can navigate).

### 3. Progressive queries — chain the result

Every `QueryRun` can be fed back into `f5q.q()`, or queried via the
method form `run.q(...)`.  The prior values become the new primary
input — `.` reads the synthesised list, so `.[]` iterates jq-style:

```python
from dialects.f5 import query as f5q

# Function form.
virtuals = f5q.q(".ltm.virtual[] | .name", "bigip.conf")
web_vses = f5q.q('.[] | select(contains(., "web"))', virtuals)

# Method form — identical semantics, reads top-down.
web_vses = (
    f5q.q(".ltm.virtual[] | .name", "bigip.conf").q('.[] | select(contains(., "web"))').q("count")
)
```

Multiple priors combine into one flat list:

```python
all_names = f5q.q(".[]", virtuals, pools)  # union
```

Mix a prior with a file and the prior binds as `$_chain` while the
file acts as primary — useful for "join my prior result against a
config":

```python
keep = f5q.q(".ltm.virtual[] | .name", "v1.conf")
matched = f5q.q(
    ".ltm.virtual[] | select(contains($_chain, .name))",
    keep,
    "v2.conf",
)
```

### 4. Get typed values back

`QueryRun` exposes the shapes external scripts normally want:

```python
from dialects.f5 import query as f5q

run = f5q.q(".ltm.virtual[]", "bigip.conf")

run.values()  # flat list of every value the query produced
run.first()  # the first value, or None
run.objects()  # only the ObjectRef instances
run.paths()  # full_path of every ObjectRef / PathRef
run.rows()  # [QueryRow(uri, value), ...] keeping the source URI
```

`ObjectRef`, `PathRef`, and `Stream` are exported from `dialects.f5.query` so
your script can pattern-match on them without reaching into private sub-modules.

### 5. Get plain JSON-compatible Python — `.out()`

Typed handles (`ObjectRef.full_path`, `.fields`, …) are great for
graph walks but they don't JSON-serialise.  `.out()` coerces:

```python
import json
from dialects.f5 import query as f5q

text = json.dumps(f5q.q(".ltm.virtual[]", "bigip.conf").out())
# ObjectRef → {"kind": ..., "full-path": ..., "fields": {...}}
# PathRef → str
# Stream → list
```

### 6. Apply a mutation and read back the rewritten config

```python
from dialects.f5 import query as f5q

run = f5q.q(
    '.ltm.virtual[].destination |= ip("192.168.9.0/24", .)',
    "bigip.conf",
)

if run.result.has_mutation:
    rewritten = run.edits("bigip.conf")
    if rewritten is not None:
        print(rewritten)  # the new file contents
```

Edits stay in memory — `f5q.q()` never writes to disk.  The CLI's
`--in-place` is implemented on top of the same API: it just calls
`Path(uri).write_text(run.edits(uri))`.

### 7. Render with a registered plugin

`QueryRun.render(name, **opts)` dispatches the values to the named
renderer plugin (built-in or user-registered):

```python
from dialects.f5 import query as f5q

# ASCII Gantt of pool-member up/down transitions.
print(
    f5q.q(
        """
        f5log_load("ltm.log")[]
        | select(.module == "01340011" or .module == "01340012")
        | tsv(.timestamp,
              (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
              (if .module == "01340011" then "DOWN" else "UP" end))
    """,
        "bigip.conf",
    ).render("gantt", **{"unit-minutes": "10"})
)
```

Three renderers ship in-tree: `gantt`, `ascii-blocks`, `mermaid`.
List them with `f5q.list_renderers()` (where `f5q = dialects.f5.query`) or `f5 q --help-renderers`.

### 8. Inline callables — the one-off escape hatch

When a custom renderer or input parser is one-shot (a script doesn't
deserve its own XDG plugin), pass a **function directly** anywhere
the API accepts a registered name:

```python
from dialects.f5 import query as f5q

# Inline renderer — a callable matching (values, **opts) -> str.
text = f5q.q(".ltm.virtual[] | .name", "bigip.conf").render(
    lambda values, **opts: ", ".join(str(v) for v in values) + "\n"
)


# Inline input parser — (source, *, uri, options=()) -> Any.
def parse_xml(source, *, uri, options=()):
    import xml.etree.ElementTree as ET

    root = ET.fromstring(source)
    return [e.attrib for e in root]


routes = f5q.load("routes.xml", parser=parse_xml)
# or in one shot:
hits = f5q.q(".items[].name", "routes.xml", parser=parse_xml)
```

Use inline callables for **one-offs**.  When the same renderer or
parser is going to be used by more than one script, register it as
an XDG plugin (next section) so the CLI and other scripts pick it
up without copy-paste.

### 9. Ship a custom renderer

```python
from dialects.f5.query import renderer


@renderer(
    "md-table",
    summary="Markdown table of results.",
    accepts="list of dicts",
)
def _render_md_table(values, **opts):
    if not values:
        return "(no rows)\n"
    headers = list(values[0].keys())
    out = ["| " + " | ".join(headers) + " |", "| " + " | ".join("---" for _ in headers) + " |"]
    for row in values:
        out.append("| " + " | ".join(str(row.get(h, "")) for h in headers) + " |")
    return "\n".join(out) + "\n"
```

### 10. Ship a custom builtin function (DSL extension)

The same engine that registers `length`, `select`, `ip`, and the
other in-tree builtins is exposed as the public `@builtin`
decorator:

```python
from dialects.f5.query import builtin


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
documented in the [plugin contract design
doc](../design/f5-query-renderer-contract.md) and modelled by the
in-tree builtins.

### 11. Ship a custom input format

```python
from dialects.f5.query import input_format


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

### 12. Auto-load plugins from `~/.config/dialects/f5/query/plugins/`

The engine scans `$XDG_CONFIG_HOME/dialects/f5/query/plugins/*.py` (default
`~/.config/dialects/f5/query/plugins/*.py`) on the first registry access.  Drop a
file in that directory and it loads transparently the next time
`f5 q` runs or any script imports a registry helper — no
`import my_plugin` ceremony, no PYTHONPATH dance:

```sh
mkdir -p ~/.config/dialects/f5/query/plugins
cat > ~/.config/dialects/f5/query/plugins/my_extensions.py <<'PY'
from dialects.f5.query import builtin, renderer, input_format

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

**Multi-file plugins** — the loader temporarily prepends each
plugin's parent directory to `sys.path` for the duration of the
import, so a top-level plugin can `import helper` to pull in a
sibling `helper.py` (or `_helper.py` — `_*.py` files are skipped
by the loader's own scan but stay importable from a plugin).  The
entry is removed after the plugin loads, so `sys.path` doesn't
accumulate state.  Top-level plugin files always load **before**
sub-folder files; within each tier files load alphabetically.

The same loader runs from Python:

```python
from dialects.f5 import query as f5q

f5q.load_user_plugins()  # idempotent — call once at startup
print(f5q.xdg_plugin_dir())  # diagnostic: where the loader looks
print(f5q.list_renderers())  # everything available after loading
```

`load_user_plugins(force=True)` re-scans for **new** files — files
that already imported successfully are skipped silently to keep the
decorator registries free of duplicate-registration noise.  Files
that previously failed are retried in case the user just fixed the
syntax error.  This makes the documented "drop a new plugin file
and reload" workflow surface only the new file in the return
value.

## How to tell it worked

```sh
uv run python -c "from dialects.f5 import query as f5q; print(len(f5q.q('.ltm.virtual[]', 'bigip.conf')))"
```

…prints the virtual-server count of the file.  `f5 q --help-renderers`
lists every renderer your script registered; `--help-builtins`,
`--help-inputs`, and `--help-plugins` cover the other three
plugin types and the XDG loader's pickup list.

## Related

- **Python API reference** — Sphinx autodoc-generated reference at
  `docs/sphinx/_build/html/index.html` (run `make docs-html`).
  The same source builds on Read the Docs via
  [`.readthedocs.yaml`](../../.readthedocs.yaml).
- [KCS index](README.md)
- [KCS: feature — `f5 query` plugins](features/kcs-feature-f5-query-renderers.md)
- [KCS: how-to — reproduce an HTTP monitor with `f5 query`](kcs-howto-reproduce-http-monitor-with-query.md)
- [`f5 query` reference manual](../references/f5_query/manual.md)
- [Design — `f5 query` plugin contract](../design/f5-query-renderer-contract.md)
