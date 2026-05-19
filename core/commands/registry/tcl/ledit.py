# Scaffolded from lreplace.n -- refine and commit
"""ledit -- In-place list edit; like lreplace but mutates a variable."""

from __future__ import annotations

from compiler.side_effects import StorageType
from compiler.types import TclType

from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl 9 man page ledit.n"


@register
class LeditCommand(CommandDef):
    name = "ledit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ledit",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Replace elements in a list variable in place",
                synopsis=("ledit listVar first last ?element element ...?",),
                snippet=(
                    "ledit takes a variable named listVar containing a list, "
                    "and replaces the range from first to last with the "
                    "element arguments.  The variable is updated and the new "
                    "value is also returned.  Introduced in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ledit listVar first last ?element element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            pure=False,
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            assigns_variable_at=0,
            arg_types={
                1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                2: ArgTypeHint(expected=TclType.INT, shimmers=True),
            },
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            side_effect_hints=(),
        )
