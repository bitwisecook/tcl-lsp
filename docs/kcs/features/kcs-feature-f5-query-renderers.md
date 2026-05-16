# KCS: feature — `f5 query` plugins (renderers, builtins, input formats)

> **Audience:** User
> **Type:** Functionality

## Summary

A decorator-registered plugin system that lets users extend `f5 query`
(alias `f5 q`) along three axes — output renderers, DSL builtin
functions, and side-input parsers — by dropping a Python file into
`$XDG_CONFIG_HOME/f5q/plugins/`.

## Applies to

tcl-lsp CLI

## Question

What do the `f5 query` renderer plugins do, and how do I use them?

## How to use

A renderer is a named output formatter the query engine dispatches to
after it has evaluated your DSL expression.  Three built-in renderers
ship in-tree, and any script that imports `f5q` can register more.

### tcl-lsp CLI

Pick a renderer with `-R / --render NAME`; pass per-renderer options
with `--render-opt KEY=VALUE` (repeatable).  List the catalogue with
`--help-renderers`:

```sh
f5 q --help-renderers
f5 q --render gantt   '<query>' bigip.conf
f5 q --render mermaid '<query>' bigip.conf --render-opt direction=TB
```

### Python (`f5q`)

```python
from f5q import Query

print(
    Query(".ltm.virtual[]")
    .run(paths=["bigip.conf"])
    .render("mermaid", direction="TB")
)
```

Custom plugins use the `@renderer` decorator and are picked up by the
CLI once the registering module is imported:

```python
from f5q import renderer

@renderer("md-table", summary="Markdown table.", accepts="list of dicts")
def _render(values, **opts):
    ...
```

See
[KCS: how-to — script against `f5 query` from Python](../kcs-howto-script-against-f5-query-from-python.md)
for the full plugin walkthrough.

## Options

- `--render NAME` — dispatch the result to renderer plugin *NAME*
  (mutually exclusive with `--scf` / `--raw` / `--paths-only` /
  `--json` / `--table` / `--table-lineart`).
- `--render-opt KEY=VALUE` — pass an option to the renderer; repeat
  for multiple options.  Renderers parse and validate their own
  options.
- `--input KIND NAME=PATH` — bind a file via input-format plugin
  *KIND*, available to the query as `$NAME` (repeatable).  The
  typed `--input-json` / `--input-jsonl` / `--input-csv` /
  `--input-f5log` shorthands cover the four built-in formats;
  `--input` is the generic shape user plugins ride.
- `--help-renderers` — print the registered renderer catalogue.
- `--help-inputs` — print the registered input-format catalogue.
- `--help-builtins [NAME]` — print every registered DSL builtin,
  including user plugins under category `user`.
- `--help-plugins` — print the XDG plugin directory and every
  plugin file the loader picked up.  Diagnostic for "is my
  plugin actually being loaded?".

## Built-in plugins

### Renderers (`--render NAME`)

| Name | Accepts | Output |
|------|---------|--------|
| `gantt` | `(timestamp, label, state)` rows | ASCII Gantt timeline with `v` / `^` / `#` glyphs |
| `ascii-blocks` | `{title, rows}` tree | Unicode line-art nested boxes |
| `mermaid` | stream of `ObjectRef` (graph mode) or any ordered stream (chain mode) | Mermaid `graph` source |

`gantt` reads `unit-minutes=N` (a divisor of 60, default 5) and
`year=YYYY`.  `ascii-blocks` reads `style=rounded|square|ascii` and
`min-width=N`.  `mermaid` reads `direction=LR|RL|TB|BT`,
`reverse=true|false`, and `max-depth=N`.

### Input formats (`--input KIND NAME=PATH`)

| Kind | Shape |
|------|-------|
| `json` | single value (dict / list / scalar) |
| `jsonl` | list of values (one per non-blank line) |
| `csv` | list of row-dicts; first row names columns unless `headers` option set |
| `f5log` | list of structured event dicts |

### Builtins

The full builtin catalogue (`length`, `select`, `map`, `ip`, `in_cidr`, `refs`,
`referenced_by`, `url_get`, …) is documented at
[`docs/references/f5_query/builtins.md`](../../references/f5_query/builtins.md)
and surfaced via `f5 q --help-builtins`.  User plugin builtins
default to category `user`.

## Writing a plugin

Three decorators cover the surface; each ships from the public
`f5q` package and registers via import-time side-effect:

```python
# ~/.config/f5q/plugins/my_extensions.py
from f5q import builtin, renderer, input_format

@builtin("uppercase", summary="ASCII uppercase.", min_args=1, max_args=1)
def _u(s):
    return str(s).upper()

@renderer("count-only", summary="Print just the row count.", accepts="any")
def _c(values, **opts):
    return f"{len(values)} rows\n"

@input_format("simple-list", summary="Newline-separated string list.")
def _l(source, *, uri, options=()):
    return [line for line in source.splitlines() if line.strip()]
```

Drop the file into `~/.config/f5q/plugins/` (or wherever
`XDG_CONFIG_HOME` points) and it loads automatically on the next
`f5 q` invocation:

```sh
f5 q --raw 'uppercase(.ltm.virtual[].name)' bigip.conf
f5 q --render count-only '.ltm.virtual[]' bigip.conf
f5 q --input simple-list svcs=services.txt '$svcs[]' bigip.conf
f5 q --help-plugins      # confirm the file loaded
```

A plugin that fails to import (syntax error, missing dependency)
emits a `f5q: warning: ...` to stderr and is skipped — the rest of
the CLI keeps working.

## Example

### Before — pipe the query output through a sidecar Python script

```sh
f5 q --raw '
    f5log_load("logs/t1-a.log")[]
    | select(.module == "01340011" or .module == "01340012")
    | tsv(.timestamp,
          (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
          (if .module == "01340011" then "DOWN" else "UP" end))
  ' bigip.conf \
  | grep -v '^#' \
  | python3 sysadmin/monitor_timeline.py
```

### After — one tool, no glue

```sh
f5 q --render gantt '
    f5log_load("logs/t1-a.log")[]
    | select(.module == "01340011" or .module == "01340012")
    | tsv(.timestamp,
          (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
          (if .module == "01340011" then "DOWN" else "UP" end))
  ' bigip.conf
```

Both forms produce the same chart:

```
members down/up over time (1 char = 5 min)
                      10          11          12          13          14
                      +----------------------------------------------------
t2_c01_vip:443        |v#^
t2_c02_vip:443        |   v######^
t2_c03_vip:443        |          v#######^
t2_c04_vip:443        |                  v#########^
...
```

## Related

- [KCS: how-to — script against `f5 query` from Python](../kcs-howto-script-against-f5-query-from-python.md)
- [KCS: how-to — reproduce an HTTP monitor with `f5 query`](../kcs-howto-reproduce-http-monitor-with-query.md)
- [Design — `f5 query` renderer contract](../../design/f5-query-renderer-contract.md)
- [KCS feature index](README.md)
