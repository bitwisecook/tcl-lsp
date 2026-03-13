# Enriched from F5 iRules reference documentation.
"""DNS::return -- Skips all further processing after TCL execution and sends the DNS packet in the opposite direction."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__return.html"


@register
class DnsReturnCommand(CommandDef):
    name = "DNS::return"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::return",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Skips all further processing after TCL execution and sends the DNS packet in the opposite direction.",
                synopsis=("DNS::return",),
                snippet=(
                    "This iRules command skips all further processing after TCL execution\n"
                    "and sends the DNS packet in the opposite direction.\n"
                    "In the DNS_REQUEST context, DNS::return signals that no further DNS\n"
                    "resolution should occur for this request upon completion of the event.\n"
                    "To provide a useful response, resource record and header changes must\n"
                    "be made in the iRule. The next event triggered is the DNS_RESPONSE\n"
                    "event.\n"
                    "In the DNS_RESPONSE context, DNS::return sends a request back for\n"
                    "additional processing.\n"
                    "\n"
                    "**Important**: `DNS::return` does NOT stop iRule execution. You must\n"
                    "follow it with `return` to prevent the rest of the event handler\n"
                    "from running."
                ),
                source=_SOURCE,
                examples=(
                    "# Send one or more IP addresses for a response to an A query\n"
                    "# Use on an LTM virtual server with a DNS profile enabled\n"
                    "when DNS_REQUEST {\n"
                    "        # Log query details\n"
                    '        log local0. "\\[DNS::question name\\]: [DNS::question name],\\\n'
                    "                \\[DNS::question class\\]: [DNS::question class],\n"
                    '                \\[DNS::question type\\]: [DNS::question type]"\n'
                    "\n"
                    "        # Generate an answer with two A records"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::return",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
