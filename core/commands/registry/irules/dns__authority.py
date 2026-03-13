# Enriched from F5 iRules reference documentation.
"""DNS::authority -- Returns, inserts, removes, or clears RRs from the authority section."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__authority.html"


_av = make_av(_SOURCE)


@register
class DnsAuthorityCommand(CommandDef):
    name = "DNS::authority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::authority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns, inserts, removes, or clears RRs from the authority section.",
                synopsis=("DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?",),
                snippet=(
                    "This iRules command returns, inserts, removes, or clears RRs from the\n"
                    "authority section.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "authority record in all responses\n"
                    "            when DNS_RESPONSE {\n"
                    '                DNS::authority insert [DNS::rr "devcentral.f5.com. 88 IN SOA 1.2.3.4"]\n'
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                    arg_values={
                        0: (
                            _av(
                                "clear",
                                "DNS::authority clear",
                                "DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                            _av(
                                "insert",
                                "DNS::authority insert",
                                "DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?",
                            ),
                            _av(
                                "remove",
                                "DNS::authority remove",
                                "DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?",
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
