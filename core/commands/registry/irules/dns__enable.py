# Enriched from F5 iRules reference documentation.
"""DNS::enable -- Sets the service state to enabled for the current DNS packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__enable.html"


@register
class DnsEnableCommand(CommandDef):
    name = "DNS::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the service state to enabled for the current DNS packet.",
                synopsis=("DNS::enable (DNS_COMPONENT)+",),
                snippet=(
                    "This iRules command sets the service state to enabled for the current\n"
                    "DNS packet.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "ns express to resolve requests from a specific ip,\n"
                    "            # disable dns express for all other requests\n"
                    "            when DNS_REQUEST {\n"
                    "                DNS::disable dnsx\n"
                    '                if { [IP::client_addr] equals "192.168.1.245" } {\n'
                    "                    DNS::enable dnsx\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::enable (DNS_COMPONENT)+",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
