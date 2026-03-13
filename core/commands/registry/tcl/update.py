# Scaffolded from update.n -- refine and commit
"""update -- Process pending events and idle callbacks."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page update.n"


@register
class UpdateCommand(CommandDef):
    name = "update"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="update",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Process pending events and idle callbacks",
                synopsis=("update ?idletasks?",),
                snippet="This command is used to bring the application by entering the event loop repeatedly until all pending events (including idle callbacks) have been processed.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="update ?idletasks?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
