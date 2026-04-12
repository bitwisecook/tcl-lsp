# Enriched from F5 iRules reference documentation.
"""DNS::log -- Controls log publisher in DNS log profile."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__log.html"


@register
class DnsLogCommand(CommandDef):
    name = "DNS::log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::log",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls log publisher in DNS log profile.",
                synopsis=("DNS::log (MESSAGE)?",),
                snippet="There are two version of this command.  DNS::log by itself returns a boolean indicating whether a DNS Logging Profile is configured in the DNS profile.  DNS::log with an argument logs a message to that log publisher.",
                source=_SOURCE,
                examples=(
                    "# Send one or more IP addresses for a response to an A query\n"
                    "            # Use on an LTM virtual server with a DNS profile enabled\n"
                    "            when DNS_REQUEST {\n"
                    "                # Log query details\n"
                    '                DNS::log "DNS question name: [DNS::question name],\n'
                    "                    DNS question class: [DNS::question class],\n"
                    '                    DNS question type: [DNS::question type]"\n'
                    "\n"
                    "                # Generate an answer with two A records"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::log (MESSAGE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
