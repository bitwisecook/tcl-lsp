# Enriched from F5 iRules reference documentation.
"""DNS::header -- Gets (v11.0+) or sets (v11.1+) simple bits or byte fields."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__header.html"


_av = make_av(_SOURCE)


@register
class DnsHeaderCommand(CommandDef):
    name = "DNS::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets (v11.0+) or sets (v11.1+) simple bits or byte fields.",
                synopsis=("DNS::header (('id' (UNSIGNED_SHORT)?)",),
                snippet=(
                    "This iRules command gets or sets simple bits or byte fields. Read-only\n"
                    "form introduced in v11.0, Read-write capability added in v11.1.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "queries from a specific ip\n"
                    "            when DNS_REQUEST {\n"
                    '                if { [IP::client_addr] equals "192.168.1.245" } {\n'
                    "                    DNS::answer clear\n"
                    "                    DNS::header rcode REFUSED\n"
                    "                    DNS::return\n"
                    "                    return\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::header (('id' (UNSIGNED_SHORT)?)",
                    arg_values={
                        0: (_av("id", "DNS::header id", "DNS::header (('id' (UNSIGNED_SHORT)?)"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
