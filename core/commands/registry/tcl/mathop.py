"""tcl::mathop -- mathematical and comparison operator commands.

These commands are the underlying implementations of Tcl's expression
operators and are made available in the global namespace via
``namespace import ::tcl::mathop::*``.
"""

from __future__ import annotations

from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _REGISTRY

_SOURCE = "Tcl man page mathop.n"

_DEFAULT_ARITY = Arity(0)

_OPERATORS: list[tuple[str, str, Arity | None]] = [
    # Arithmetic
    ("+", "addition", None),
    ("-", "subtraction or negation", Arity(1)),
    ("*", "multiplication", None),
    ("/", "division", Arity(1)),
    ("%", "modulo", Arity(2, 2)),
    ("**", "exponentiation", None),
    # Bitwise
    ("&", "bitwise AND", None),
    ("|", "bitwise OR", None),
    ("^", "bitwise XOR", None),
    ("~", "bitwise NOT (unary)", Arity(1, 1)),
    ("<<", "left shift", Arity(2, 2)),
    (">>", "right shift", Arity(2, 2)),
    # Comparison
    ("==", "numeric equality", None),
    ("!=", "numeric inequality", Arity(2, 2)),
    ("<", "numeric less-than", None),
    (">", "numeric greater-than", None),
    ("<=", "numeric less-or-equal", None),
    (">=", "numeric greater-or-equal", None),
    ("eq", "string equality", None),
    ("ne", "string inequality", Arity(2, 2)),
    ("in", "list membership", Arity(2, 2)),
    ("ni", "list non-membership", Arity(2, 2)),
    # Logical
    ("!", "logical NOT (unary)", Arity(1, 1)),
    ("&&", "logical AND", None),
    ("||", "logical OR", None),
    # List index
    ("@", "list index operator", Arity(2, 2)),
    # Min / max
    ("min", "minimum of arguments", Arity(1)),
    ("max", "maximum of arguments", Arity(1)),
]


def _make_op_class(op_name: str, summary: str, arity: Arity | None) -> type[CommandDef]:
    """Create a CommandDef subclass for a single mathop operator."""
    op_spec = CommandSpec(
        name=op_name,
        dialects=DIALECTS_EXCEPT_IRULES,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(f"{op_name} ?arg ...?",),
            snippet=(
                f"The {op_name} operator command. Available via namespace import ::tcl::mathop::*."
            ),
            source=_SOURCE,
        ),
        forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=f"{op_name} ?arg ...?"),),
        validation=ValidationSpec(arity=arity or _DEFAULT_ARITY),
    )

    class OpCmd(CommandDef):
        name = op_name

        @classmethod
        def spec(cls) -> CommandSpec:
            return op_spec

    OpCmd.__name__ = f"Mathop_{op_name}"
    OpCmd.__qualname__ = OpCmd.__name__
    return OpCmd


for _name, _summary, _arity in _OPERATORS:
    _REGISTRY.append(_make_op_class(_name, _summary, _arity))
    # Also register the fully-qualified spellings.  The bare names
    # are used by the expression compiler (prefix-form codegen);
    # the namespace-qualified spellings are needed when scripts
    # invoke the operators as ordinary commands, e.g.
    # ``[::tcl::mathop::== $a $b $c]`` in upstream tcltest fixtures
    # and the chained-comparison idiom.  The Zig runtime registers
    # both spellings in ``runtime/zig/cmds/tcl_mathop.zig``; pairing
    # them here keeps the WASM parity gate (``orphan_builtins``)
    # green.
    _REGISTRY.append(_make_op_class(f"::tcl::mathop::{_name}", _summary, _arity))
    _REGISTRY.append(_make_op_class(f"tcl::mathop::{_name}", _summary, _arity))
