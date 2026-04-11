"""disconnect -- Disconnect the process from the controlling terminal."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect disconnect(1)"


@register
class DisconnectCommand(CommandDef):
    name = "disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="disconnect",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Disconnect the process from the controlling terminal (daemonise).",
                synopsis=("disconnect",),
                snippet=(
                    "Disconnects the forked process from the terminal. "
                    "Typically used after ``fork`` in the child process."
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="disconnect"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )
