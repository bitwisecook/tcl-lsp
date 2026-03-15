"""exp_continue -- Continue matching in an expect body."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exp_continue(1)"


@register
class ExpContinueCommand(CommandDef):
    name = "exp_continue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exp_continue",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Continue matching within an expect body instead of returning.",
                synopsis=("exp_continue ?-continue_timer?",),
                snippet=(
                    "Used inside an ``expect`` body to re-enter the pattern "
                    "matching loop. With ``-continue_timer``, the timeout "
                    "timer is not restarted."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exp_continue ?-continue_timer?",
                    options=(
                        OptionSpec(
                            name="-continue_timer",
                            detail="Do not restart the timeout timer.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )
