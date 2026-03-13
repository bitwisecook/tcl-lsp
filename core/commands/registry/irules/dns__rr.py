# Enriched from F5 iRules reference documentation.
"""DNS::rr -- Creates a new resource record object with specified attributes or as a complete string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__rr.html"


@register
class DnsRrCommand(CommandDef):
    name = "DNS::rr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::rr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new resource record object with specified attributes or as a complete string.",
                synopsis=(
                    "DNS::rr ANY_CHARS",
                    "DNS::rr NAME DNS_TYPE DNS_CLASS TTL RDATA",
                ),
                snippet=(
                    "This iRules command creates a new resource record object with specified\n"
                    "attributes or as a complete string.\n"
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
                    synopsis="DNS::rr ANY_CHARS",
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
