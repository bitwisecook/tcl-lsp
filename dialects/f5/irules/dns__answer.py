# Enriched from F5 iRules reference documentation.
"""DNS::answer -- Returns, inserts, removes, or clears all RRs from the answer section."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__answer.html"


_av = make_av(_SOURCE)


@register
class DnsAnswerCommand(CommandDef):
    name = "DNS::answer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::answer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns, inserts, removes, or clears all RRs from the answer section.",
                synopsis=("DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?",),
                snippet=(
                    "This iRules command returns, inserts, removes, or clears RRs from the\n"
                    "answer section.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "ttl of all answer records and add a glue record\n"
                    "            when DNS_RESPONSE {\n"
                    "                set rrs [DNS::answer]\n"
                    "                foreach rr $rrs {\n"
                    "                    DNS::ttl $rr 1234\n"
                    "                }\n"
                    '                set new_rr [DNS::rr "bigip3900-30.f5net.com. 88 IN A 1.2.3.4"]\n'
                    "                DNS::additional insert $new_rr\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::answer ?clear | insert <rr> | remove <rr>?",
                    arg_values={
                        0: (
                            _av("clear", "Clear all answer RRs.", "DNS::answer clear"),
                            _av(
                                "insert",
                                "Insert an RR into the answer section.",
                                "DNS::answer insert <rr_object>",
                            ),
                            _av(
                                "remove",
                                "Remove an RR from the answer section.",
                                "DNS::answer remove <rr_object>",
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
                "clear": SubCommand(
                    name="clear",
                    arity=Arity(0, 0),
                    detail="Clear all answer RRs.",
                    synopsis="DNS::answer clear",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.DNS_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(1, 1),
                    detail="Insert an RR into the answer section.",
                    synopsis="DNS::answer insert <rr_object>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.DNS_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1, 1),
                    detail="Remove an RR from the answer section.",
                    synopsis="DNS::answer remove <rr_object>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.DNS_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
