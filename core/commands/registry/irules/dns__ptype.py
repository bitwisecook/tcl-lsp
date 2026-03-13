# Enriched from F5 iRules reference documentation.
"""DNS::ptype -- Returns the type of the DNS packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__ptype.html"


@register
class DnsPtypeCommand(CommandDef):
    name = "DNS::ptype"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::ptype",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the type of the DNS packet.",
                synopsis=("DNS::ptype",),
                snippet=(
                    "This iRules command returns the type of the DNS packet.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "OMAIN response is going to be sent,\n"
                    "            # instead attach a record to resolve to.\n"
                    "            when DNS_RESPONSE {\n"
                    '                if { [DNS::ptype] == "NXDOMAIN" } {\n'
                    "                    DNS::header rcode NOERROR\n"
                    '                    DNS::answer insert "[DNS::question name]. 60 [DNS::question class] [DNS::question type] 192.168.1.245"\n'
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::ptype",
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
