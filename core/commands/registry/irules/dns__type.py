# Enriched from F5 iRules reference documentation.
"""DNS::type -- Gets or sets the resource record type field."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__type.html"


@register
class DnsTypeCommand(CommandDef):
    name = "DNS::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the resource record type field.",
                synopsis=("DNS::type RR_OBJECT (DNS_TYPE)?",),
                snippet=(
                    "This iRules command gets or sets the resource record type field\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    "        set rrs [DNS::answer]\n"
                    "        foreach rr $rrs {\n"
                    '            if { [DNS::type $rr] == "SOA" } {\n'
                    "                DNS::answer remove $rr\n"
                    "            }\n"
                    "        }\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::type RR_OBJECT (DNS_TYPE)?",
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
