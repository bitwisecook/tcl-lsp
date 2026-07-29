# KCS: How do I add a third-party Tcl library to the command registry?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

VS Code, Zed, JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI

## Question

How do I add first-class registry support for a third-party Tcl package
(hover docs, completion, arity checks, side-effect classification, taint
hints, call-graph integration) — the same way `tcllib` is wired in?

## Before you start

- The target package is widely used enough to justify being in the
  shipped registry rather than declared per-project via
  [stubs](kcs-howto-annotate-commands-with-stubs.md).
- You can enumerate the package's commands (names, arities, argument
  shapes) from upstream docs.
- You have a checkout of the repo and can run `make ci-fast` + the
  Python test suite locally.

## Answer

The registry pattern is one Python file per package (or per
namespace within a package), each file declaring one `CommandDef`
subclass per command. The classes register themselves via a
`@register` decorator at import time; the package's `__init__.py`
exposes a `<pkg>_command_specs()` factory that
`CommandRegistry.build_default()` (or `load_dialect_specs()`) calls.

This walkthrough uses `sqlite3` — the same package used as the
running stub example elsewhere — as a concrete target.

### 1. Decide the home

Pick the directory under `dialects/` that matches the
package's distribution shape:

| Distribution shape | Folder | Loader |
|---|---|---|
| Bundled with Tcl core | `dialects/tcl/` | always available |
| Standard library package (Tk, http, msgcat, …) | `dialects/stdlib/` | gated by `package require` |
| tcllib package | `dialects/tcllib/` | gated by `package require` |
| Dialect-specific (iRules, iApps, EDA vendors, Expect) | `dialects/f5/irules/`, `dialects/f5/iapps/`, `dialects/eda/`, `dialects/expect/` | loaded on dialect activation |
| Standalone C extension (sqlite3, tdom, …) | `dialects/stdlib/` (alongside Tk and friends) | gated by `package require` |

`sqlite3` is a standalone C extension that needs `package require
sqlite3`, so it belongs under `dialects/stdlib/`.

### 2. Create the package module

Add `dialects/stdlib/sqlite3_.py` (the trailing
underscore matches the convention used for module names that would
otherwise clash with the standard library or built-ins):

```python
"""sqlite3 — SQLite Tcl bindings."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import (
    ConnectionSide,
    SideEffect,
    SideEffectTarget,
)
from ._base import register

_SOURCE = "sqlite3 Tcl bindings"
_PACKAGE = "sqlite3"
_BODY = frozenset({ArgRole.BODY})


@register
class Sqlite3Command(CommandDef):
    """The factory command — ``sqlite3 dbName ?path? ?options?``
    creates the instance command ``dbName``."""

    name = "sqlite3"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            required_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Open or create a SQLite database and create an instance command.",
                synopsis=("sqlite3 dbName ?path? ?-create boolean? ?-readonly boolean? ...",),
                source=_SOURCE,
                examples="sqlite3 db :memory:",
                return_value="The name of the new instance command.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sqlite3 dbName ?path? ?options...?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(SideEffect(target=SideEffectTarget.FILE_IO, writes=True),),
        )
```

That gives `sqlite3` itself a real spec: hover docs, completion, arity
check, and a side-effect classification. But it does not yet model
`db eval` / `db transaction` / `db function` callbacks — those live on
the *instance command*, which is created dynamically. See section 4.

### 3. Wire it into the loader

Add the module to `dialects/stdlib/__init__.py`:

```python
from . import sqlite3_  # noqa: F401
```

and confirm the existing `stdlib_command_specs()` factory walks the
shared `_REGISTRY` list — no other change is needed; the `@register`
decorator does the work.

If you're adding a new dialect rather than a package, follow the
existing pattern in `command_registry.py:99` — add an entry to
`_DIALECT_LOADER_SPECS` so `load_dialect_specs()` can find your
factory.

### 4. Model instance-command callbacks

`sqlite3 db :memory:` creates a command named `db`. The instance name
is user-chosen, so we can't put a literal `db` spec in the registry —
the registry only knows fixed names. There are two strategies:

**Strategy A — handle it at the analyser level.** Detect `sqlite3
$name ?path?` invocations and synthesise an instance command with the
right signature into the per-analysis overlay. This is similar in
spirit to how `oo::class create Foo` is recognised. It requires a
small lowering hook in `compiler/lowering_hooks/` that watches
for `sqlite3` calls and registers `$name` with a `SubcommandSig`
overlay carrying the `eval` / `transaction` / `function` /
`onecolumn` / `cache` subcommands. The overlay shape mirrors the
existing built-in ensemble commands (`dict`, `string`, `info`).

**Strategy B — ship a sqlite-extras file users opt into.** If the
canonical instance name is conventional (`db` in many sqlite
projects), expose a helper that registers the well-known names via
`stub_signature_scope` automatically when `package require sqlite3`
is seen. Lower correctness ceiling but no lowering-hook needed.

Strategy A is what the existing `tcloo` / `snit` integrations use
for class-created instance commands; Strategy B is a stop-gap. For
sqlite3 the recommended long-term path is A; until then users can
continue to use [stubs](kcs-howto-annotate-commands-with-stubs.md)
to declare their instance command shapes.

### 5. Add side-effect, taint, and type metadata

`CommandSpec` carries optional fields that drive other analyses:

```python
return CommandSpec(
    name="sqlite3::dbName::eval",
    required_package=_PACKAGE,
    forms=(...),
    validation=ValidationSpec(arity=Arity(1, 3)),
    arg_roles={2: _BODY},
    side_effect_hints=(SideEffect(target=SideEffectTarget.FILE_IO, reads=True, writes=True),),
    evaluates_code=True,  # script arg is treated as Tcl code
    creates_dynamic_barrier=True,  # callback body crosses analysis boundary
)
```

| Field | What it drives |
|---|---|
| `arg_roles` / `arg_role_resolver` | Where script bodies, variables, channels, patterns live. Feeds the call-graph scanner, var-usage analyser, and lexer. |
| `validation.arity` | Arity diagnostics (W101 / E120). |
| `side_effect_hints` | Purity propagation, dead-store elimination, IRule taint flow. |
| `evaluates_code` | Marks the command as a script-runner like `eval` / `uplevel`. |
| `creates_dynamic_barrier` | Signals analysis boundary for SSA / variable-escape. |
| `assigns_variable_at` | Variable-write detection for commands like `set`, `lassign`. |
| `creates_scope_alias` | Upvar-style aliasing. |
| `const_fold` | A callable that constant-folds the command at compile time. |
| `taint_hints()` (class method) | Per-command taint flow, declared via override on `CommandDef`. |
| `tcllib_package` / `required_package` | Gates the command on a matching `package require`. |

The full field reference is in `compiler/registry/models.py`
and the design notes in `docs/design/compiler/command-registry.md`.

### 6. Add tests

Each new package gets a focused test file under `tests/`:

```python
# tests/test_registry_sqlite3.py
from compiler.registry import REGISTRY

def test_sqlite3_factory_registered():
    spec = REGISTRY.get_any("sqlite3")
    assert spec is not None
    assert spec.required_package == "sqlite3"

def test_arity_diagnostic_fires_on_zero_args():
    # ... call the analyser, assert W101 raised
```

End-to-end coverage (call graph, hover, completion) belongs in
`tests/test_semantic_graph.py`, `tests/test_hover.py`, and
`tests/test_completion.py` respectively — extend those rather than
duplicating fixtures.

### 7. Refresh derived caches

After running, `make snapshot-wasm-parity` updates the WASM parity
baseline to acknowledge the new commands. `make gen-editor-settings`
regenerates the per-editor diagnostic catalogues. Commit both
alongside the registry change — CI will fail if they're stale.

## How to tell it worked

- `python -c "from compiler.registry import REGISTRY; print(REGISTRY.get_any('sqlite3'))"`
  returns a `CommandSpec`, not `None`.
- A Tcl file containing `package require sqlite3` followed by
  `sqlite3 db :memory:` no longer raises the W315 "unresolved command"
  diagnostic on `sqlite3`.
- Hovering `sqlite3` in VS Code shows the synopsis from the
  `HoverSnippet`.
- `tcl callgraph` on a sqlite-using file shows edges into row callbacks
  *without* a `# tcl-lsp: stub` block in the source — the registry now
  knows the command shape directly.

## Related

- [How to annotate an external Tcl command with a stub](kcs-howto-annotate-commands-with-stubs.md)
  — the lighter-weight, per-project alternative when registry inclusion
  isn't warranted.
- [Command registry design doc](../design/compiler/command-registry.md)
- [CommandSpec field reference](../GLOSSARY.md#commandspec)
- [KCS index](README.md)
