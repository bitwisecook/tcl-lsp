# Enriched from F5 iRules reference documentation.
"""DNS::answer -- Returns, inserts, removes, or clears all RRs from the answer section."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
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
                    synopsis="DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                    arg_values={
                        0: (
                            _av(
                                "clear",
                                "DNS::answer clear",
                                "DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                            _av(
                                "insert",
                                "DNS::answer insert",
                                "DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                            _av(
                                "remove",
                                "DNS::answer remove",
                                "DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?",
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
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
