# Enriched from F5 iRules reference documentation.
"""timing -- Enables or disables iRule timing statistics."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/timing.html"


@register
class TimingCommand(CommandDef):
    name = "timing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables or disables iRule timing statistics.",
                synopsis=("timing TIMING",),
                snippet=(
                    "The timing command can be used to enable iRule timing statistics. This\n"
                    "will then collect timing information as specified each time the rule is\n"
                    'evaluated. Statistics may be viewed with "b rule show all" or in the\n'
                    "Statistics tab of the iRules Editor.\n"
                    "\n"
                    "Note: In 11.5.0, timing was enabled by default for all iRules in\n"
                    "BZ375905. The performance impact is negligible. As a result, you no\n"
                    "longer need to use this command to view timing statistics."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    ...\n  }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="timing TIMING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
