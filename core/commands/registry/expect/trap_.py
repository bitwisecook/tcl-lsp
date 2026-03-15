"""trap -- Trap signals."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect trap(1)"


@register
class TrapCommand(CommandDef):
    name = "trap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="trap",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Trap signals and execute a command when they occur.",
                synopsis=(
                    "trap ?command? ?signal ...?",
                    "trap SIG_IGN SIGINT",
                    "trap { puts caught } SIGTERM",
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="trap ?command? ?signal ...?"),),
            validation=ValidationSpec(arity=Arity(0)),
            arg_roles={0: ArgRole.BODY},
        )
