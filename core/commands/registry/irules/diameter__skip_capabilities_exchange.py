# Enriched from F5 iRules reference documentation.
"""DIAMETER::skip_capabilities_exchange -- Instructs DIAMETER protocol to skip capabilities exchange when establishing a peering relationship."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__skip_capabilities_exchange.html"


@register
class DiameterSkipCapabilitiesExchangeCommand(CommandDef):
    name = "DIAMETER::skip_capabilities_exchange"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::skip_capabilities_exchange",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Instructs DIAMETER protocol to skip capabilities exchange when establishing a peering relationship.",
                synopsis=("DIAMETER::skip_capabilities_exchange ( HOSTNAME )?",),
                snippet=(
                    "Once called, the current connection will skip DIAMETER capabilities exchange message communication with the peer device and will immediately be able to receive DIAMETER messaegs.\n"
                    "\n"
                    "If the HOSTNAME parameter is provided, the provided name will be used as the peer device's origin-host attribute for logging."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '                if { ([IP::address] starts_with "192.168.") } {\n'
                    "                    DIAMETER::skip_capabilities_exchange [IP::address].somesp.com\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::skip_capabilities_exchange ( HOSTNAME )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
