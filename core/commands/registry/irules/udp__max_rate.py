# Enriched from F5 iRules reference documentation.
"""UDP::max_rate -- This command can be used to set/get the maximum transmission rate (bytes per second) of a UDP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__max_rate.html"


@register
class UdpMaxRateCommand(CommandDef):
    name = "UDP::max_rate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::max_rate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the maximum transmission rate (bytes per second) of a UDP connection.",
                synopsis=("UDP::max_rate (UDP_MAX_RATE)?",),
                snippet=(
                    "UDP::max_rate returns the maximum transmission rate (bytes per second) of a UDP connection.\n"
                    "UDP::max_rate UDP_MAX_RATE sets the maximum transmission rate (bytes per second) to specified value.\n"
                    "UDP::max_rate 0 turns off the maximum transmission rate (bytes per second) of a previously specified value."
                ),
                source=_SOURCE,
                examples=(
                    "# Get/set the max rate of the UDP flow.\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    # Set the rate to 1Mbps (125,000 bytes per second)\n"
                    '    log local0. "UDP set max rate: [UDP::max_rate 125000]"\n'
                    '    log local0. "UDP get max rate: [UDP::max_rate]"\n'
                    "}"
                ),
                return_value="UDP::max_rate returns the maximum transmission rate (bytes per second) of a UDP connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::max_rate (UDP_MAX_RATE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
