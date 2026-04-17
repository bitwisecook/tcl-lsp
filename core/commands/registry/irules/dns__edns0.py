# Enriched from F5 iRules reference documentation.
"""DNS::edns0 -- Gets (v11.0+) and sets (v11.1+) the values of the edns0 pseudo-RR."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__edns0.html"


_av = make_av(_SOURCE)


@register
class DnsEdns0Command(CommandDef):
    name = "DNS::edns0"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::edns0",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets (v11.0+) and sets (v11.1+) the values of the edns0 pseudo-RR.",
                synopsis=(
                    "DNS::edns0 'remove' ('nsid' | 'subnet')?",
                    "DNS::edns0 'exists' ('nsid' | 'subnet')?",
                    "DNS::edns0 'do' (BOOLEAN)?",
                    "DNS::edns0 'sz' (UNSIGNED_SHORT)?",
                ),
                snippet=(
                    "This iRules command gets (v11.0+) and sets (v11.1+) the values of the\n"
                    "edns0 pseudo-RR.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_REQUEST {\n"
                    "  if { [DNS::edns0 exists] } {\n"
                    '    log local0. [DNS::edns0 subnet address]"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::edns0 'remove' ('nsid' | 'subnet')?",
                    arg_values={
                        0: (
                            _av(
                                "remove",
                                "DNS::edns0 remove",
                                "DNS::edns0 'remove' ('nsid' | 'subnet')?",
                            ),
                            _av(
                                "nsid",
                                "DNS::edns0 nsid",
                                "DNS::edns0 'remove' ('nsid' | 'subnet')?",
                            ),
                            _av(
                                "subnet",
                                "DNS::edns0 subnet",
                                "DNS::edns0 'remove' ('nsid' | 'subnet')?",
                            ),
                            _av(
                                "exists",
                                "DNS::edns0 exists",
                                "DNS::edns0 'exists' ('nsid' | 'subnet')?",
                            ),
                            _av("do", "DNS::edns0 do", "DNS::edns0 'do' (BOOLEAN)?"),
                            _av("sz", "DNS::edns0 sz", "DNS::edns0 'sz' (UNSIGNED_SHORT)?"),
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
