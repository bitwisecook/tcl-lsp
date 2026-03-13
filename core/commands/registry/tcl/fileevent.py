# Scaffolded from fileevent.n -- refine and commit
"""fileevent -- Execute a script when a channel becomes readable or writable."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page fileevent.n"


@register
class FileeventCommand(CommandDef):
    name = "fileevent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fileevent",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Execute a script when a channel becomes readable or writable",
                synopsis=(
                    "fileevent channel readable ?script?",
                    "fileevent channel writable ?script?",
                ),
                snippet="The fileevent command has been superceded by the chan event command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fileevent channel readable ?script?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 3),
            ),
            arg_roles={2: ArgRole.BODY},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
