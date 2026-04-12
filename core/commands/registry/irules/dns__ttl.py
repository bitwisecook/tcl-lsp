# Enriched from F5 iRules reference documentation.
"""DNS::ttl -- Gets or sets the resource record TTL field."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__ttl.html"


@register
class DnsTtlCommand(CommandDef):
    name = "DNS::ttl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::ttl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the resource record TTL field.",
                synopsis=("DNS::ttl RR_OBJECT (TTL)?",),
                snippet=(
                    "This iRules command gets or sets the resource record TTL field\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    "        set rrs [DNS::answer]\n"
                    "        foreach rr $rrs {\n"
                    "            DNS::ttl $rr 1234\n"
                    "        }\n"
                    '        set new_rr [DNS::rr "bigip3900-30.f5net.com. 88 IN A 1.2.3.4"]\n'
                    "        DNS::additional insert $new_rr\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::ttl RR_OBJECT (TTL)?",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
