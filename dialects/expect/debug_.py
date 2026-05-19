"""debug -- Enable or disable Expect debugger."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect debug(1)"


@register
class DebugCommand(CommandDef):
    name = "debug"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="debug",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Enable or disable the Expect debugger.",
                synopsis=("debug ?-now? ?0 | 1?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="debug ?-now? ?0 | 1?",
                    options=(OptionSpec(name="-now", detail="Enter debugger immediately."),),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
