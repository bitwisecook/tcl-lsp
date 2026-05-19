# Scaffolded from rename.n -- refine and commit
"""rename -- Rename or delete a command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page rename.n"


@register
class RenameCommand(CommandDef):
    name = "rename"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="rename",
            is_language_keyword=True,
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
            arg_roles={0: frozenset({ArgRole.NAME}), 1: frozenset({ArgRole.NAME})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
