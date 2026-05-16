---
myst:
  html_meta:
    description: f5q Python API reference — autodoc-generated from docstrings.
---

# API reference

Everything `f5q` exports.  Generated from the docstrings on each
public symbol, so this page can't drift from `import f5q`.

## Calling the engine

The polymorphic single-call entry plus the pre-staging helper most
scripts use.

```{eval-rst}
.. autofunction:: f5q.q
.. autofunction:: f5q.load
```

## Result types

The wrapper types `q()` / `load()` return and the typed values that
appear inside a result.

```{eval-rst}
.. autoclass:: f5q.QueryRun
   :members:
   :special-members: __len__, __bool__, __iter__, __getitem__
```

```{eval-rst}
.. autoclass:: f5q.QueryRow
   :members:
```

```{eval-rst}
.. autoclass:: f5q.Sources
   :members:
```

```{eval-rst}
.. autoclass:: f5q.ObjectRef
   :members:
```

```{eval-rst}
.. autoclass:: f5q.PathRef
   :members:
```

```{eval-rst}
.. autoclass:: f5q.Stream
   :members:
```

```{eval-rst}
.. autoclass:: f5q.Root
   :members:
```

## Lower-level entry points

`q()` and `load()` cover most scripts.  These lower-level entries
stay available for code that needs explicit control over source
maps, named bindings, partitions, or merge mode.

```{eval-rst}
.. autoclass:: f5q.Query
   :members:
.. autoclass:: f5q.QueryResult
   :members:
.. autofunction:: f5q.run_query
.. autofunction:: f5q.parse_query
.. autoclass:: f5q.InputSpec
   :members:
```

## Plugin registries

The three decorators an extension uses to register itself, plus
the helpers that surface the catalogue.

### Renderers

Format an evaluator result as a string — Mermaid graph, ASCII
Gantt, Unicode line-art block tree, custom.

```{eval-rst}
.. autofunction:: f5q.renderer
.. autofunction:: f5q.render
.. autofunction:: f5q.list_renderers
```

### DSL builtins

Add new functions callable from the query language
(`my_func(.x, .y)`).

```{eval-rst}
.. autofunction:: f5q.builtin
.. autofunction:: f5q.list_builtins
.. autofunction:: f5q.format_builtins
```

### Input formats

Parse a non-BIG-IP side-input format (`--input KIND NAME=PATH` on
the CLI, `parser=` on `load()` / `q()` from Python).

```{eval-rst}
.. autofunction:: f5q.input_format
.. autofunction:: f5q.list_input_formats
```

## XDG plugin auto-loader

Drop a Python file into `$XDG_CONFIG_HOME/f5q/plugins/` and it
loads transparently on the first registry access — no
`import my_plugin` ceremony, no PYTHONPATH dance.

```{eval-rst}
.. autofunction:: f5q.xdg_plugin_dir
.. autofunction:: f5q.load_user_plugins
```

## DSL grammar and worked examples

Helpers that surface the same reference material the CLI prints
via `f5 query --help-dsl` / `--help-examples`.  Useful for
embedding the DSL grammar into a host application's help system.

```{eval-rst}
.. autofunction:: f5q.format_grammar
.. autofunction:: f5q.format_examples
.. autofunction:: f5q.list_examples
```

## Exceptions

Every error the engine raises descends from `QueryError`, so a
script catching the umbrella type stays robust to engine changes.

```{eval-rst}
.. autoexception:: f5q.QueryError
.. autoexception:: f5q.LexError
.. autoexception:: f5q.ParseError
.. autoexception:: f5q.EvalError
.. autoexception:: f5q.EditError
.. autoexception:: f5q.BuiltinError
.. autoexception:: f5q.RendererError
```
