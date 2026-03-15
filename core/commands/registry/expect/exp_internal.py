"""exp_internal -- Control Expect internal diagnostics."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exp_internal(1)"


@register
class ExpInternalCommand(CommandDef):
    name = "exp_internal"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exp_internal",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control Expect internal diagnostic output.",
                synopsis=("exp_internal ?-f file? 0|1",),
                snippet=(
                    "With ``1``, Expect prints diagnostic information about "
                    "pattern matching and other internal activity. Useful for "
                    "debugging scripts."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exp_internal ?-f file? 0|1",
                    options=(
                        OptionSpec(
                            name="-f",
                            takes_value=True,
                            value_hint="file",
                            detail="Log diagnostics to the specified file.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
