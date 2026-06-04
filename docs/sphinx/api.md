---
myst:
  html_meta:
    description: f5q Python API reference — autodoc-generated from docstrings.
---

# API reference

Everything `dialects.f5.query` exports.  Generated from the docstrings on each
public symbol, so this page can't drift from `import dialects.f5.query`.

## Calling the engine

The polymorphic single-call entry plus the pre-staging helper most
scripts use.

```{eval-rst}
.. autofunction:: dialects.f5.query.q
.. autofunction:: dialects.f5.query.load
```

## Result types

The wrapper types `q()` / `load()` return and the typed values that
appear inside a result.

```{eval-rst}
.. autoclass:: dialects.f5.query.QueryRun
   :members:
   :special-members: __len__, __bool__, __iter__, __getitem__
```

```{eval-rst}
.. autoclass:: dialects.f5.query.QueryRow
   :members:
```

```{eval-rst}
.. autoclass:: dialects.f5.query.Sources
   :members:
```

```{eval-rst}
.. autoclass:: dialects.f5.query.ObjectRef
   :members:
```

```{eval-rst}
.. autoclass:: dialects.f5.query.PathRef
   :members:
```

```{eval-rst}
.. autoclass:: dialects.f5.query.Stream
   :members:
```

```{eval-rst}
.. autoclass:: dialects.f5.query.Root
   :members:
```

## Lower-level entry points

`q()` and `load()` cover most scripts.  These lower-level entries
stay available for code that needs explicit control over source
maps, named bindings, partitions, or merge mode.

```{eval-rst}
.. autoclass:: dialects.f5.query.Query
   :members:
.. autoclass:: dialects.f5.query.QueryResult
   :members:
.. autofunction:: dialects.f5.query.run_query
.. autofunction:: dialects.f5.query.parse_query
.. autoclass:: dialects.f5.query.InputSpec
   :members:
```

## Plugin registries

The three decorators an extension uses to register itself, plus
the helpers that surface the catalogue.

### Renderers

Format an evaluator result as a string — Mermaid graph, ASCII
Gantt, Unicode line-art block tree, custom.

```{eval-rst}
.. autofunction:: dialects.f5.query.renderer
.. autofunction:: dialects.f5.query.render
.. autofunction:: dialects.f5.query.list_renderers
```

### DSL builtins

Add new functions callable from the query language
(`my_func(.x, .y)`).

```{eval-rst}
.. autofunction:: dialects.f5.query.builtin
.. autofunction:: dialects.f5.query.list_builtins
.. autofunction:: dialects.f5.query.format_builtins
```

### Input formats

Parse a non-BIG-IP side-input format (`--input KIND NAME=PATH` on
the CLI, `parser=` on `load()` / `q()` from Python).

```{eval-rst}
.. autofunction:: dialects.f5.query.input_format
.. autofunction:: dialects.f5.query.list_input_formats
```

## XDG plugin auto-loader

Drop a Python file into `$XDG_CONFIG_HOME/dialects/f5/query/plugins/` and it
loads transparently on the first registry access — no
`import my_plugin` ceremony, no PYTHONPATH dance.

```{eval-rst}
.. autofunction:: dialects.f5.query.xdg_plugin_dir
.. autofunction:: dialects.f5.query.load_user_plugins
```

## DSL grammar and worked examples

Helpers that surface the same reference material the CLI prints
via `f5 query --help-dsl` / `--help-examples`.  Useful for
embedding the DSL grammar into a host application's help system.

```{eval-rst}
.. autofunction:: dialects.f5.query.format_grammar
.. autofunction:: dialects.f5.query.format_examples
.. autofunction:: dialects.f5.query.list_examples
```

## Exceptions

Every error the engine raises descends from `QueryError`, so a
script catching the umbrella type stays robust to engine changes.

```{eval-rst}
.. autoexception:: dialects.f5.query.QueryError
.. autoexception:: dialects.f5.query.LexError
.. autoexception:: dialects.f5.query.ParseError
.. autoexception:: dialects.f5.query.EvalError
.. autoexception:: dialects.f5.query.EditError
.. autoexception:: dialects.f5.query.BuiltinError
.. autoexception:: dialects.f5.query.RendererError
```
