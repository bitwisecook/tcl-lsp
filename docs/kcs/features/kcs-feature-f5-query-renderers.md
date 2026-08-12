# KCS: feature — `f5 query` renderers, builtins, and input formats

> **Audience:** User
> **Type:** Functionality

## Summary

`f5 query` (alias `f5 q`) reaches beyond plain output along three axes —
output renderers that format the result, DSL builtin functions callable
from the query language, and side-input parsers that read non-BIG-IP
files into the query.

## Applies to

tcl-lsp CLI

## Question

What renderers, builtins, and input formats does `f5 query` offer, and
how do I use them?

## How to use

- **Renderers** turn evaluator output into a formatted string — a
  Mermaid diagram, an ASCII Gantt timeline, a line-art block tree.
- **Builtins** are DSL functions callable from the query language
  (`length(.x)`, `in_cidr(.y, "10.0.0.0/8")`).
- **Input formats** parse non-BIG-IP side-inputs (`--input csv
  routes=routes.csv`).

```sh
# Renderers.
f5 q --help-renderers
f5 q --render gantt   '<query>' bigip.conf
f5 q --render mermaid '<query>' bigip.conf --render-opt direction=TB

# Input formats — generic --input KIND NAME=PATH.
f5 q --help-inputs
f5 q --input csv routes=routes.csv '$routes[].name' bigip.conf

# Builtins — listed with the rest of the DSL builtins.
f5 q --help-builtins
f5 q --raw 'length(.ltm.virtual[])' bigip.conf
```

## Options

- `--render NAME` — dispatch the result to renderer *NAME*
  (mutually exclusive with `--scf` / `--raw` / `--paths-only` /
  `--json` / `--table` / `--table-lineart`).
- `--render-opt KEY=VALUE` — pass an option to the renderer; repeat
  for multiple options.  Renderers parse and validate their own
  options.
- `--input KIND NAME=PATH` — bind a file parsed as *KIND*, available
  to the query as `$NAME` (repeatable).  The typed `--input-json` /
  `--input-jsonl` / `--input-csv` / `--input-f5log` shorthands are
  the same thing for the four commonest formats.
- `--help-renderers` — print the renderer catalogue.
- `--help-inputs` — print the input-format catalogue.
- `--help-builtins [NAME]` — print every DSL builtin, or just *NAME*.

## What ships

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
| `zone` | DNS zone file — list of resource-record dicts |

### Builtins

The full builtin catalogue (`length`, `select`, `map`, `ip`, `in_cidr`, `refs`,
`referenced_by`, `url_get`, …) is documented at
[`docs/references/f5_query/builtins.md`](../../references/f5_query/builtins.md)
and surfaced via `f5 q --help-builtins`.

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

- [KCS: how-to — reproduce an HTTP monitor with `f5 query`](../kcs-howto-reproduce-http-monitor-with-query.md)
- [KCS: how-to — compose query streams](../kcs-howto-compose-query-streams.md)
- [Design — `f5 query` plugin contract](../../design/f5-query-renderer-contract.md)
- [KCS feature index](README.md)
