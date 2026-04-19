# Enriched from F5 iRules reference documentation.
"""DNS::rdata -- Gets or sets the resource record rdata field."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__rdata.html"


@register
class DnsRdataCommand(CommandDef):
    name = "DNS::rdata"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::rdata",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the resource record rdata field.",
                synopsis=("DNS::rdata RR_OBJECT (VALUE)?",),
                snippet=(
                    "This iRules command gets or sets the resource record rdata field\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    "         set rrs [DNS::answer]\n"
                    "         foreach rr $rrs {\n"
                    '             log local0. "[DNS::rdata $rr]"\n'
                    "         }\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::rdata RR_OBJECT (VALUE)?",
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
