# Enriched from F5 iRules reference documentation.
"""TCP::lossfilterburst -- Gets the TCP Loss Ignore Burst Parameter."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__lossfilterburst.html"


@register
class TcpLossfilterburstCommand(CommandDef):
    name = "TCP::lossfilterburst"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::lossfilterburst",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the TCP Loss Ignore Burst Parameter.",
                synopsis=("TCP::lossfilterburst",),
                snippet=(
                    "Gets the maximum size burst loss (in packets) before triggering congestion response.\n"
                    "  * Burst range is valid from 0 to 32. Higher values decrease the\n"
                    "    chance of performing congestion control."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    # Set loss filter burst to a maximum of 3\n"
                    "    if { [TCP::lossfilterburst] > 3 } {\n"
                    "        TCP::lossfilter [TCP::lossfilterrate] 3\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP Loss Ignore Burst in packets.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::lossfilterburst",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
