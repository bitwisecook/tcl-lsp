# Enriched from F5 iRules reference documentation.
"""DNS::name -- Gets or sets the resource record name field."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__name.html"


@register
class DnsNameCommand(CommandDef):
    name = "DNS::name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::name",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the resource record name field.",
                synopsis=("DNS::name RR_OBJECT (VALUE)?",),
                snippet=(
                    "This iRules command gets or sets the resource record name field.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "s responses returned to a specific client ip\n"
                    "            when DNS_RESPONSE {\n"
                    '                if { [IP::client_addr] equals "192.168.1.245" } {\n'
                    "                    DNS::log [DNS::name [DNS::answer]]\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::name RR_OBJECT (VALUE)?",
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
