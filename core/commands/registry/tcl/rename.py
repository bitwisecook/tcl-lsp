# Scaffolded from rename.n -- refine and commit
"""rename -- Rename or delete a command."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page rename.n"


@register
class RenameCommand(CommandDef):
    name = "rename"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="rename",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Rename or delete a command",
                synopsis=("rename oldName newName",),
                snippet="Rename the command that used to be called oldName so that it is now called newName.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="rename oldName newName",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            arg_roles={0: ArgRole.NAME, 1: ArgRole.NAME},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
