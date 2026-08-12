# KCS: How do I use the `f5 query` engine from a Python script?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

I want to run the same jq-flavoured BIG-IP queries that `f5 query` runs,
but from inside my own Python script — to feed a custom report, plot the
results, or loop over many configs — without shelling out to the CLI and
parsing stdout.

## Before you start

- The `f5report` package installed in the same Python environment as your
  script. It is a native extension module built with
  [maturin](https://www.maturin.rs/) from `rust/bigip-report-gen/python`;
  running `maturin develop` in that directory builds it and installs the
  package into the active virtual environment. Python 3.9 or newer.
- A `bigip.conf`, SCF, or `.ucs` file to query.
- Familiarity with the query language — run `f5 query --help-dsl` once, or
  skim the [reference manual](../references/f5_query/manual.md).

## Answer

The query engine itself is Rust. `f5report` binds it straight into Python,
so a query runs in-process — no subprocess, no stdout parsing, and no
second copy of the config parser. Two functions do nearly all the work.

### 1. Load the configs

`load_paths` reads each path and hands back a list of `(uri, text)` pairs.
A `.ucs` archive is unpacked in memory to its SCF text; add
`passphrase="…"` for an encrypted one.

```python
import f5report

sources = f5report.load_paths(["device-01.ucs", "device-02.bigip.conf"])
```

Anywhere a script already holds the config text, skip `load_paths` — every
call below also accepts a `{uri: text}` dictionary or a list of
`(uri, text)` pairs.

### 2. Run a query

```python
names = f5report.query(".ltm.virtual[] | .name", sources)
```

The result is a flat list of ordinary Python values across every source.
A configuration object arrives as a dictionary with `kind`, `full-path`,
and `fields` keys; a path reference arrives as its full path string; a
stream arrives as a list.

The keyword arguments cover the rest of the engine:

- `per_file=True` returns `[(uri, [values]), …]` in source order rather
  than one flat list.
- `merge=True` treats every loaded config as a single namespace, so
  reference walks such as `referenced_by(.)` cross file boundaries. Two
  sources defining the same object are refused.
- `partitions={uri: "Partition"}` tells the loader which BIG-IP partition
  a source belongs to. The default is `Common`.
- `enable_probes=True` opts in to the live network-probe functions. Left
  off, they refuse to touch the network.
- `side_inputs=[(name, kind, text), …]` parses extra data and binds it to
  `$name` in the query. The kinds are `csv`, `f5log`, `json`, `jsonl`,
  and `zone`.

### 3. Catch the one exception

Everything the engine rejects — a syntax error, an unknown function, a
merge collision — raises `f5report.QueryError`:

```python
try:
    rows = f5report.query(".ltm.virtual[", sources)
except f5report.QueryError as exc:
    print(f"query failed: {exc}")
```

### 4. Rewrites and rendered output stay on the CLI

The binding is deliberately read-only: an expression that assigns or
renames raises `QueryError` rather than rewriting anything. For a rewrite,
use `f5 query --write` (to stdout) or `f5 query --in-place`. The rendered
output modes are a CLI surface too — `f5 query --render gantt`,
`--render ascii-blocks`, and `--render mermaid`, described in
[the renderer note](features/kcs-feature-f5-query-renderers.md).

### 5. Or skip straight to the report

The same package builds the full interactive HTML report the `f5-report`
command produces:

```python
html = f5report.build_report(sources, title="Production LTM")
```

## How to tell it worked

```sh
python -c "import f5report; print(f5report.query('.ltm.virtual[] | .name', f5report.load_paths(['bigip.conf'])))"
```

…prints the virtual-server names in the file. `f5report.engine_version()`
reports the version of the engine the binding was built from.

## Related

- [KCS index](README.md)
- [KCS: feature — `f5 query` renderers and input formats](features/kcs-feature-f5-query-renderers.md)
- [KCS: how-to — reproduce an HTTP monitor with `f5 query`](kcs-howto-reproduce-http-monitor-with-query.md)
- [`f5 query` reference manual](../references/f5_query/manual.md)
- [Design — `f5 query` renderer, builtin, and input-format contract](../design/f5-query-renderer-contract.md)
