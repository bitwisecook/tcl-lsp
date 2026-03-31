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
    ("-", "subtraction or negation", None),
    ("*", "multiplication", None),
    ("/", "division", None),
    ("%", "modulo", None),
    ("**", "exponentiation", None),
    # Bitwise
    ("&", "bitwise AND", None),
    ("|", "bitwise OR", None),
    ("^", "bitwise XOR", None),
    ("~", "bitwise NOT (unary)", None),
    ("<<", "left shift", None),
    (">>", "right shift", None),
    # Comparison
    ("==", "numeric equality", None),
    ("!=", "numeric inequality", None),
    ("<", "numeric less-than", None),
    (">", "numeric greater-than", None),
    ("<=", "numeric less-or-equal", None),
    (">=", "numeric greater-or-equal", None),
    ("eq", "string equality", None),
    ("ne", "string inequality", None),
    ("in", "list membership", None),
    ("ni", "list non-membership", None),
    # Logical
    ("!", "logical NOT (unary)", None),
    ("&&", "logical AND", None),
    ("||", "logical OR", None),
    # List index
    ("@", "list index operator", Arity(2, 2)),
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
