# Enriched from F5 iRules reference documentation.
"""DIAMETER::state -- Returns the current state of the Diameter peer's connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__state.html"


@register
class DiameterStateCommand(CommandDef):
    name = "DIAMETER::state"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::state",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current state of the Diameter peer's connection.",
                synopsis=("DIAMETER::state",),
                snippet=(
                    "This iRule command returns the current state of the Diameter peer\\'s\n"
                    "connection, as a string. There are five possible states:\n"
                    "  * CLOSED - The connection is down\n"
                    "  * WAIT_ICEA - still waiting for the initial CEA\n"
                    "  * ROPEN - The connection has been reopened\n"
                    "  * IOPEN - The connection is open for the first time\n"
                    "  * CLOSING - The connection will soon be down"
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    if { [DIAMETER::state] == "ROPEN" } {\n'
                    '        log local0. "Received a DIAMETER message via a reopened connection"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::state",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
