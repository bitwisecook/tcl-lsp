# Enriched from F5 iRules reference documentation.
"""DNS::origin -- Returns the originator of the DNS message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__origin.html"


@register
class DnsOriginCommand(CommandDef):
    name = "DNS::origin"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::origin",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the originator of the DNS message.",
                synopsis=("DNS::origin",),
                snippet=(
                    "Returns the last module to modify the DNS message. Return values:\n"
                    "\n"
                    "CLIENT\n"
                    "\n"
                    "   This message has just been received by the BigIP from a client's query,\n"
                    "   and nothing has been processed.\n"
                    "\n"
                    "SERVER\n"
                    "\n"
                    "   This message has just been received by the BigIP from a server's\n"
                    "   response to a DNS query, such as On-Box or Off-Box BIND, or another\n"
                    "   BigIP entirely.\n"
                    "\n"
                    "CACHE\n"
                    "\n"
                    "   This message is a response from the DNS Cache.\n"
                    "\n"
                    "RPZ\n"
                    "\n"
                    "   This message is a response from the Response Policy Zone in your BigIP.\n"
                    "   It was blocked and either NXDOMAIN or a Walled Garden was returned as a\n"
                    "   response."
                ),
                source=_SOURCE,
                examples=(
                    "equests that were not resolved by DNS Express\n"
                    "            when DNS_RESPONSE {\n"
                    '                if { [DNS::origin] ne "DNSX" } {\n'
                    "                    DNS::drop\n"
                    "                }\n"
                    "            }"
                ),
                return_value="CLIENT SERVER CACHE GTM_BUILD GTM_REWRITE DNSX DNSSEC LAST_ACTION TCL RPZ RATE_LIMITER",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::origin",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
