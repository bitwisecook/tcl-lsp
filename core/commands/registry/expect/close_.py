"""close -- Close a connection to a spawned process (Expect override)."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect close(1)"


@register
class CloseCommand(CommandDef):
    name = "close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="close",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Close the connection to the current spawned process.",
                synopsis=("close ?-slave? ?-i spawn_id?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="close ?-slave? ?-i spawn_id?",
                    options=(
                        OptionSpec(name="-slave", detail="Close the slave side of the pty."),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Close the specified spawn id.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
