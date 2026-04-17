# Scaffolded from cd.n -- refine and commit
"""cd -- Change working directory."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page cd.n"


@register
class CdCommand(CommandDef):
    name = "cd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="cd",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Change working directory",
                synopsis=("cd ?dirName?",),
                snippet="Change the current working directory to dirName, or to the home directory (as specified in the HOME environment variable) if dirName is not given.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="cd ?dirName?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
