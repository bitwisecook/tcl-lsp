# Enriched from F5 iRules reference documentation.
"""DOSL7::slowdown -- Adds source IP extracted from current connection to greylist."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__slowdown.html"


@register
class Dosl7SlowdownCommand(CommandDef):
    name = "DOSL7::slowdown"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::slowdown",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Adds source IP extracted from current connection to greylist.",
                synopsis=("DOSL7::slowdown RATE TIMEOUT",),
                snippet=(
                    "Adds source IP extracted from current connection to greylist. TCP slowdown will be applied according to supplied RATE (in percents) and TIMEOUT (in seconds).\n"
                    "A RATE represents amount of incoming data packets to be dropped to perform slowdown."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '                 if { [HTTP::uri] contains "heavy.php" } {\n'
                    "                     DOSL7::slowdown 30 60\n"
                    "                 }\n"
                    "             }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::slowdown RATE TIMEOUT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DOSL7_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
