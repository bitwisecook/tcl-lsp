# Adding a Tcl Command

Every Tcl command lives in its own file under `core/commands/registry/tcl/`.
This guide covers every feature available in a command definition.

## Quick start

Create a file, e.g. `mycommand.py`:

```python
"""mycommand -- One-line summary of what it does."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

@register
class MycommandCommand(CommandDef):
    name = "mycommand"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="mycommand",
            hover=HoverSnippet(
                summary="One-line summary.",
                synopsis=("mycommand arg ?options?",),
                snippet="A sentence or two of extra context.",
                source="Tcl mycommand(1)",
            ),
            forms=(
                FormSpec(kind=FormKind.DEFAULT, synopsis="mycommand arg ?options?"),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
```

Then add `from . import mycommand  # noqa: F401` to `__init__.py`
(alphabetical order) and run the tests.

If the file name collides with a Python keyword or builtin, append an
underscore: `for_.py`, `set_.py`, `exec_.py`.

## File naming

| Tcl command   | File name       | Class name           |
|---------------|-----------------|----------------------|
| `set`         | `set_.py`       | `SetCommand`         |
| `oo::class`   | `oo_class.py`   | `OoClassCommand`     |
| `string`      | `string.py`     | `StringCommand`      |

## CommandSpec fields

### hover (HoverSnippet)

Shown when the user hovers over the command name.

```python
hover=HoverSnippet(
    summary="One-sentence description.",
    synopsis=("cmd arg1 ?arg2?", "cmd -server callback port"),
    snippet="Extra paragraph of context.",
    source="Tcl cmd(1)",
)
```

- `synopsis` is a tuple of usage lines (supports multiple forms).
- `source` is rendered as italic attribution text.

### forms (tuple of FormSpec)

Each `FormSpec` represents one invocation form.  Most commands have a
single `FormKind.DEFAULT` form.  Commands with getter/setter semantics
use `FormKind.GETTER` and `FormKind.SETTER` with per-form arity.

### validation (ValidationSpec)

Arity checking.  Args are counted after the command name.

```python
validation=ValidationSpec(arity=Arity(1, 3))
```

For subcommand commands, list per-subcommand arity:

```python
validation=ValidationSpec(
    arity=Arity(1),
    subcommands={
        "length": Arity(1, 1),
        "compare": Arity(2),
    },
)
```

### Behavioral traits

Set these on `CommandSpec` when the command has special structural
properties used by the compiler, formatter, or analyser:

```python
CommandSpec(
    name="proc",
    never_inline_body=True,    # formatter: don't inline the body
    has_loop_body=True,        # optimiser: body is a loop
    creates_dynamic_barrier=True,  # compiler: prevents optimisation
)
```

| Trait                      | Used by     | Set on                             |
|----------------------------|-------------|------------------------------------|
| `never_inline_body`        | formatter   | proc, if, for, foreach, switch, try, while, namespace, dict |
| `has_loop_body`            | optimiser   | for, foreach, while                |
| `creates_dynamic_barrier`  | compiler    | eval, uplevel, upvar, trace, unknown |

## Options (OptionSpec)

Add options to a `FormSpec` for completion and hover:

```python
from ..models import OptionSpec

FormSpec(
    kind=FormKind.DEFAULT,
    synopsis="lsort ?options? list",
    options=(
        OptionSpec(name="-ascii"),
        OptionSpec(name="-nocase"),
        OptionSpec(name="-command", takes_value=True, value_hint="cmdPrefix"),
        OptionSpec(name="--"),
    ),
)
```

For rich per-option hover (see `socket_.py` for the full example):

```python
OptionSpec(
    name="-async",
    detail="Connect asynchronously.",
    hover=HoverSnippet(
        summary="Create the socket immediately and complete connect asynchronously.",
        synopsis=("socket -async host port",),
        source="Tcl socket(3tcl)",
    ),
)
```

## Option terminator profiles (OptionTerminatorSpec)

Drives the W304 diagnostic ("missing `--`").  Add to `CommandSpec`:

```python
from ..models import OptionTerminatorSpec

CommandSpec(
    name="regexp",
    option_terminator_profiles=(
        OptionTerminatorSpec(
            scan_start=0,
            options_with_values=frozenset({"-start"}),
            warn_without_terminator=True,   # always warn, not just dynamic values
        ),
    ),
)
```

For subcommand-scoped profiles (e.g. `string match` but not `string length`):

```python
OptionTerminatorSpec(scan_start=1, subcommand="match"),
```

## Argument values (ArgumentValueSpec)

Provide completions and hover for specific argument positions.

### Subcommand names (arg index 0)

```python
from ..models import ArgumentValueSpec

_SUBCOMMANDS = (
    ArgumentValueSpec(value="length", detail="Return string length."),
    ArgumentValueSpec(value="match", detail="Test glob-style pattern."),
)

FormSpec(
    kind=FormKind.DEFAULT,
    synopsis="string option arg ?arg ...?",
    arg_values={0: _SUBCOMMANDS},
)
```

### Enumerated values at other positions

```python
_ACCESS_MODES = (
    ArgumentValueSpec(value="r", detail="Read only (default)."),
    ArgumentValueSpec(value="w", detail="Write only. Truncate or create."),
)

FormSpec(
    kind=FormKind.DEFAULT,
    synopsis="open fileName ?access?",
    arg_values={1: _ACCESS_MODES},
)
```

### Subcommand-specific argument values

For values that depend on which subcommand is active (e.g. `string is`
character classes), add `arg_values` to the `SubCommand` entry:

```python
_IS_CLASSES = (
    ArgumentValueSpec(value="alnum", detail="Any alphabet or digit."),
    ArgumentValueSpec(value="boolean", detail="Valid boolean value."),
)

# In the subcommands dict:
subcommands={
    "is": SubCommand(
        name="is",
        arity=Arity(2),
        detail="Test if string is a member of a character class.",
        arg_values={0: _IS_CLASSES},   # arg 0 after "string is"
    ),
}
```

### Rich hovers on argument values

```python
def _av(value, detail, synopsis=""):
    return ArgumentValueSpec(
        value=value,
        detail=detail,
        hover=HoverSnippet(
            summary=detail,
            synopsis=(synopsis,) if synopsis else (),
            source="Tcl man page string.n",
        ),
    )
```

## Role hints

Tell the compiler/analyser what role each argument plays.  Set
`arg_roles` on the `CommandSpec` (for simple commands) or on individual
`SubCommand` entries (for subcommand commands).

### Available roles (ArgRole)

| Role           | Meaning                                    |
|----------------|--------------------------------------------|
| `BODY`         | Tcl script body (recursively analysed)     |
| `EXPR`         | Expression (`expr` sub-language)           |
| `VAR_NAME`     | Variable name (target of set/incr/unset)   |
| `PARAM_LIST`   | Procedure parameter list                   |
| `NAME`         | Symbolic name (proc/namespace name)        |
| `PATTERN`      | Pattern or regex                           |
| `CHANNEL`      | Channel identifier                         |
| `INDEX`        | List/string index expression               |

### Simple command

```python
from ..signatures import ArgRole, Arity

CommandSpec(
    name="proc",
    arg_roles={
        0: ArgRole.NAME,         # proc name
        1: ArgRole.PARAM_LIST,   # args
        2: ArgRole.BODY,         # body
    },
    validation=ValidationSpec(arity=Arity(3, 3)),
    ...
)
```

### Subcommand command

```python
subcommands={
    "eval": SubCommand(
        name="eval",
        arity=Arity(2),
        arg_roles={0: ArgRole.NAME, 1: ArgRole.BODY},
    ),
    "exists": SubCommand(
        name="exists",
        arity=Arity(1, 1),
    ),
}
```

## Type hints

Tell the shimmer analyser what internal representation a command
expects for each argument and what it returns.  Set `return_type` and
`arg_types` on the `CommandSpec` (for simple commands) or on individual
`SubCommand` entries (for subcommand commands).

### Available types (TclType)

| Type        | Meaning                             |
|-------------|-------------------------------------|
| `STRING`    | Tcl string                          |
| `INT`       | Integer                             |
| `DOUBLE`    | Floating point                      |
| `BOOLEAN`   | Boolean                             |
| `LIST`      | List                                |
| `DICT`      | Dictionary                          |
| `BYTEARRAY` | Raw byte array                     |
| `NUMERIC`   | Abstract join of INT and DOUBLE     |

### Simple command

```python
from ....compiler.types import TclType
from ..models import ArgTypeHint

CommandSpec(
    name="llength",
    return_type=TclType.INT,
    arg_types={
        0: ArgTypeHint(expected=TclType.LIST, shimmers=True),
    },
    ...
)
```

- `return_type`: what the command produces.
- `shimmers=True`: the command forces conversion, destroying the
  previous internal representation.  This drives the S100/S101
  shimmer diagnostics.

### Subcommand command

```python
from ....compiler.types import TclType

subcommands={
    "length": SubCommand(
        name="length",
        arity=Arity(1, 1),
        return_type=TclType.INT,
    ),
    "exists": SubCommand(
        name="exists",
        arity=Arity(1, 1),
        return_type=TclType.BOOLEAN,
    ),
}
```

## Dialect-specific commands

By default commands are available in all dialects.  To restrict:

```python
CommandSpec(
    name="lmap",
    dialects=frozenset({"tcl8.6", "tcl9.0"}),
    ...
)
```

## Import rules

Command class files **must** import from `..signatures` and
`..type_hints`, never from `..runtime`.  This avoids circular imports
since `runtime.py` imports from the `tcl` package.

## Scaffolding

To generate an initial class file from a Tcl man page:

```bash
python scripts/registry/scaffold_tcl_commands.py \
    --doc-dir /path/to/tcl9/doc --command mycommand
```

The scaffolder skips commands that already have a class file.  Use
`--force` to regenerate (writes to `.scaffolded` for diffing).

## Checklist

When adding or enriching a command, ensure:

- [ ] File created with `@register` decorator
- [ ] Import added to `__init__.py` (alphabetical)
- [ ] `spec()` returns `CommandSpec` with hover, forms, validation
- [ ] `arg_roles` set on `CommandSpec` or `SubCommand` entries
- [ ] `return_type` / `arg_types` set on `CommandSpec` or `SubCommand` entries
- [ ] Options added to `FormSpec` if the command takes switches
- [ ] `OptionTerminatorSpec` added if command takes `--`
- [ ] `ArgumentValueSpec` added for enumerated argument values
- [ ] Behavioral traits set if applicable (never_inline_body, etc.)
- [ ] Tests pass: `pytest tcl-lsp/tests/ -x -q`
