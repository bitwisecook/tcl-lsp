# Enriched from F5 iRules reference documentation.
"""DNS::header -- Gets (v11.0+) or sets (v11.1+) simple bits or byte fields."""

# Introduced: BIG-IP v10+ (core DNS iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__header.html"


_av = make_av(_SOURCE)

_HEADER_FIELDS = frozenset(
    {
        "id",
        "qr",
        "opcode",
        "aa",
        "tc",
        "rd",
        "ra",
        "ad",
        "cd",
        "rcode",
        "qdcount",
        "ancount",
        "nscount",
        "arcount",
    }
)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.DNS_STATE,
        reads=True,
        connection_side=ConnectionSide.BOTH,
    ),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.DNS_STATE,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)

# Bit fields (0 or 1 args: get or set)
_BIT_FIELDS = {"qr", "aa", "tc", "rd", "ra", "ad", "cd"}
# Numeric fields (0 or 1 args: get or set)
_NUM_FIELDS = {"id", "opcode", "rcode", "qdcount", "ancount", "nscount", "arcount"}


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
                synopsis=(
                    "DNS::header <field> ?value?",
                    "DNS::header id ?UNSIGNED_SHORT?",
                    "DNS::header rcode ?RCODE_VALUE?",
                ),
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
                    synopsis="DNS::header <field> ?value?",
                    arg_values={
                        0: (
                            _av("id", "Get/set message ID.", "DNS::header id ?value?"),
                            _av("qr", "Get/set query/response flag.", "DNS::header qr ?0|1?"),
                            _av("opcode", "Get/set operation code.", "DNS::header opcode ?value?"),
                            _av("aa", "Get/set authoritative answer flag.", "DNS::header aa ?0|1?"),
                            _av("tc", "Get/set truncation flag.", "DNS::header tc ?0|1?"),
                            _av("rd", "Get/set recursion desired flag.", "DNS::header rd ?0|1?"),
                            _av("ra", "Get/set recursion available flag.", "DNS::header ra ?0|1?"),
                            _av("ad", "Get/set authenticated data flag.", "DNS::header ad ?0|1?"),
                            _av("cd", "Get/set checking disabled flag.", "DNS::header cd ?0|1?"),
                            _av("rcode", "Get/set response code.", "DNS::header rcode ?value?"),
                            _av(
                                "qdcount", "Get/set question count.", "DNS::header qdcount ?value?"
                            ),
                            _av("ancount", "Get/set answer count.", "DNS::header ancount ?value?"),
                            _av(
                                "nscount", "Get/set authority count.", "DNS::header nscount ?value?"
                            ),
                            _av(
                                "arcount",
                                "Get/set additional count.",
                                "DNS::header arcount ?value?",
                            ),
                        )
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
            subcommands={
                field: SubCommand(
                    name=field,
                    arity=Arity(0, 1),
                    detail=f"Get/set the {field} header field.",
                    synopsis=f"DNS::header {field} ?value?",
                    pure=True,
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                )
                for field in _HEADER_FIELDS
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
