# KCS: feature — `f5 query` renderer plugins

> **Audience:** User
> **Type:** Functionality

## Summary

A decorator-registered plugin system that lets `f5 query` (alias
`f5 q`) dispatch result values to a named renderer — Mermaid diagram,
ASCII Gantt timeline, or Unicode line-art block diagram — and lets
Python scripts ship custom renderers in one line.

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

- `--render NAME` — dispatch the result to plugin *NAME* (mutually
  exclusive with `--scf` / `--raw` / `--paths-only` / `--json` /
  `--table` / `--table-lineart`).
- `--render-opt KEY=VALUE` — pass an option to the renderer; repeat
  for multiple options.  Renderers parse and validate their own
  options.
- `--help-renderers` — print the registered renderer catalogue.

## Built-in renderers

| Name | Accepts | Output |
|------|---------|--------|
| `gantt` | `(timestamp, label, state)` rows | ASCII Gantt timeline with `v` / `^` / `#` glyphs |
| `ascii-blocks` | `{title, rows}` tree | Unicode line-art nested boxes |
| `mermaid` | stream of `ObjectRef` (graph mode) or any ordered stream (chain mode) | Mermaid `graph` source |

`gantt` reads `unit-minutes=N` (a divisor of 60, default 5) and
`year=YYYY`.  `ascii-blocks` reads `style=rounded|square|ascii` and
`min-width=N`.  `mermaid` reads `direction=LR|RL|TB|BT`,
`reverse=true|false`, and `max-depth=N`.

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
