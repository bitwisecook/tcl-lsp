"""tkwait -- Wait for a variable, visibility, or window change."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tk man page tkwait.n"


@register
class TkwaitCommand(CommandDef):
    name = "tkwait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tkwait",
            required_package="Tk",
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Wait for a variable to be written, a window to be destroyed, or a window to become visible.",
                synopsis=(
                    "tkwait variable name",
                    "tkwait visibility name",
                    "tkwait window name",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tkwait variable|visibility|window name",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            arg_roles={1: frozenset({ArgRole.VAR_READ})},
        )
