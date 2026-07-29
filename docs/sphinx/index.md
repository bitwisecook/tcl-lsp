# dialects.f5.query — Python API for the `f5 query` engine

**dialects.f5.query** is the importable Python surface of the same jq-flavoured
BIG-IP query engine that powers the `f5 query` (alias `f5 q`) CLI.
External scripts can run queries, build them up progressively,
render results through plugins or inline callables, and ship
reusable extensions (renderers, DSL builtins, side-input parsers)
via one-line decorators auto-loaded from
`$XDG_CONFIG_HOME/dialects/f5/query/plugins/`.

```python
from dialects.f5.query import q

# One-liner — first non-file string is the expression, everything
# else is an input.
for name in q(".ltm.virtual[] | .name", "bigip.conf"):
    print(name)

# Progressive — chain queries.  The prior values become the new
# primary input; the typed wrapper stays immutable.
filtered = q(".ltm.virtual[]", "bigip.conf").q(".[] | select(.pool != null)").q(".[] | .name")

# Render with a built-in plugin OR an inline callable.
filtered.render("ascii-blocks")
filtered.render(lambda values, **opts: ", ".join(map(str, values)) + "\n")

# Coerce to plain JSON-compatible Python.
data = filtered.out()
```

## What's on this site

This site is the **API reference** for `dialects.f5.query` — autodoc-generated
from the docstrings on every public symbol, so it can't drift from
what `import dialects.f5.query` actually exposes.

For **narrative material** (tutorials, the DSL grammar reference,
worked recipes, plugin packaging walkthroughs) follow the
**External documentation** links lower on this page.

```{toctree}
:maxdepth: 2
:caption: API reference

api
```

```{toctree}
:maxdepth: 1
:caption: Project

changelog
```

## Installing

`f5q` ships as part of the `tcl-lsp` package:

```sh
uv pip install tcl-lsp                  # released wheel
uv pip install -e path/to/tcl-lsp       # editable checkout
```

The CLI (`f5 q`) ships as a standalone zipapp — see
[INSTALL-cli.md](https://github.com/bitwisecook/tcl-lsp/blob/main/INSTALL-cli.md).

## At-a-glance

| Topic | Entry point |
|---|---|
| Run a query in one call | {py:func}`dialects.f5.query.q` |
| Pre-stage source files | {py:func}`dialects.f5.query.load` |
| Chain queries progressively | {py:meth}`dialects.f5.query.QueryRun.q` |
| Get typed values back | {py:meth}`dialects.f5.query.QueryRun.values`, {py:meth}`dialects.f5.query.QueryRun.objects` |
| Coerce to plain Python | {py:meth}`dialects.f5.query.QueryRun.out` |
| Render with a plugin or callable | {py:meth}`dialects.f5.query.QueryRun.render` |
| Ship a custom renderer | {py:func}`dialects.f5.query.renderer` |
| Ship a custom DSL function | {py:func}`dialects.f5.query.builtin` |
| Ship a custom input format | {py:func}`dialects.f5.query.input_format` |
| XDG auto-load directory | {py:func}`dialects.f5.query.xdg_plugin_dir`, {py:func}`dialects.f5.query.load_user_plugins` |

## External documentation

- **[KCS: how-to — script against `f5 query` from Python](https://github.com/bitwisecook/tcl-lsp/blob/main/docs/kcs/kcs-howto-script-against-f5-query-from-python.md)**
  — task-oriented walkthrough: one-liner, pre-staging, progressive
  chaining, inline callables, custom plugins, XDG auto-load.
- **[KCS: feature — `f5 query` plugins](https://github.com/bitwisecook/tcl-lsp/blob/main/docs/kcs/features/kcs-feature-f5-query-renderers.md)**
  — feature catalogue with CLI flag reference.
- **[Design — `f5 query` plugin contract](https://github.com/bitwisecook/tcl-lsp/blob/main/docs/design/f5-query-renderer-contract.md)**
  — formal contracts, registration lifecycle, error mapping.
- **[`f5 query` DSL reference](https://github.com/bitwisecook/tcl-lsp/blob/main/docs/references/f5_query/manual.md)**
  — grammar, every builtin function, sample configurations.

## Indices

- {ref}`genindex`
- {ref}`modindex`
- {ref}`search`
