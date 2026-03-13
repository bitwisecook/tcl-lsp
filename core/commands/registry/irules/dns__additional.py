# Enriched from F5 iRules reference documentation.
"""DNS::additional -- Returns, inserts, removes, or clears RRs from the additional section."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__additional.html"


_av = make_av(_SOURCE)


@register
class DnsAdditionalCommand(CommandDef):
    name = "DNS::additional"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::additional",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns, inserts, removes, or clears RRs from the additional section.",
                synopsis=("DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?",),
                snippet=(
                    "This iRules command returns, inserts, removes, or clears RRs from the\n"
                    "additional section.\n"
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
                    synopsis="DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                    arg_values={
                        0: (
                            _av(
                                "clear",
                                "DNS::additional clear",
                                "DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                            _av(
                                "insert",
                                "DNS::additional insert",
                                "DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                            _av(
                                "remove",
                                "DNS::additional remove",
                                "DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                        )
                    },
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
