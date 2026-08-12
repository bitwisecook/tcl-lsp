# KCS: feature — `f5 query` renderers and input formats

> **Audience:** User
> **Type:** Functionality

## Summary

Render `f5 query` results as a chart or diagram instead of text, and feed
the query non-BIG-IP data from a CSV, JSON, log, or DNS zone file.

## Applies to

tcl-lsp CLI

## Question

What do the `f5 query` renderers and input formats do, and how do I use
them?

## How to use

Both are catalogues built into the `f5 query` binary. A **renderer** turns
the query's output values into a formatted string; an **input format**
parses a non-BIG-IP file and binds it to a `$name` the query can read.
Neither is user-extensible at run time — there is no plugin directory to
drop a file into. Ask the binary what it has:

```sh
f5 query --help-renderers
f5 query --help-inputs
f5 query --help-builtins
```

Then use them:

```sh
# Renderers.
f5 query --render gantt '<query>' bigip.conf
f5 query --render mermaid '<query>' bigip.conf --render-opt direction=TB

# Input formats — bind services.csv to $svcs.
f5 query --input csv svcs=services.csv '$svcs[].name' bigip.conf
```

## Options

- `--render NAME` (or `-R NAME`) — send the result to renderer *NAME*.
  Overrides `--scf`, `--raw`, `--paths-only`, `--json`, `--table`, and
  `--table-lineart`.
- `--render-opt KEY=VALUE` — pass one option to the renderer. Repeat for
  several. Each renderer validates its own options.
- `--input KIND NAME=PATH` — bind a file parsed as *KIND*, readable as
  `$NAME` (repeatable). `--input-json`, `--input-jsonl`, `--input-csv`,
  and `--input-f5log` are shorthands for the same thing.
- `--help-renderers`, `--help-inputs`, `--help-builtins [NAME]` — print
  the matching catalogue and exit.

## The catalogues

### Renderers

| Name | Accepts | Output |
|---|---|---|
| `ascii-blocks` | a `{title, rows}` tree | Unicode line-art nested boxes |
| `gantt` | rows of `(timestamp, label, state)` | ASCII timeline with `v`, `^`, and `#` glyphs |
| `mermaid` | any ordered stream | a Mermaid `graph` of the values, chained |

`ascii-blocks` reads `style` (`rounded`, `square`, or `ascii`) and
`min-width`. `gantt` reads `unit-minutes` (a divisor of 60, default 5)
and `year`. `mermaid` reads `direction` (`LR`, `RL`, `TB`, or `BT`),
`reverse`, and `max-depth`.

For a diagram of the BIG-IP reference graph rather than the query's own
values, use `f5 graph --format mermaid` — it owns the config text it
walks, which the renderer does not see.

### Input formats

| Kind | Shape |
|---|---|
| `csv` | list of row-dicts; the first row names the columns unless you supply headers |
| `f5log` | list of structured BIG-IP log-event dicts |
| `json` | one value (dict, list, or scalar) |
| `jsonl` | list of values, one per non-blank line |
| `zone` | list of DNS resource records from an RFC 1035 zone file |

### Builtins

The DSL's builtin functions (`length`, `select`, `map`, `ip`, `in_cidr`,
`refs`, `referenced_by`, `url_get`, and the rest) are documented in
[`docs/references/f5_query/builtins.md`](../../references/f5_query/builtins.md)
and listed by `f5 query --help-builtins`.

## Example

### Before — pipe the query output through a sidecar script

```sh
f5 query --raw '
    f5log_load("logs/t1-a.log")[]
    | select(.module == "01340011" or .module == "01340012")
    | tsv(.timestamp,
          (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
          (if .module == "01340011" then "DOWN" else "UP" end))
  ' bigip.conf \
  | grep -v '^#' \
  | ./monitor_timeline
```

### After — one tool, no glue

```sh
f5 query --render gantt '
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
- [Design — `f5 query` renderer, builtin, and input-format contract](../../design/f5-query-renderer-contract.md)
- [KCS feature index](README.md)
