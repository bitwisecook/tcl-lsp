# Enriched from F5 iRules reference documentation.
"""DNS::len -- Returns the DNS packet message length."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__len.html"


@register
class DnsLenCommand(CommandDef):
    name = "DNS::len"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::len",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the DNS packet message length.",
                synopsis=("DNS::len",),
                snippet=(
                    "This iRules command returns the DNS packet message length.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "ngth of each packet\n"
                    "            when DNS_RESPONSE {\n"
                    '                DNS::log "PacketLength: " [DNS::len]\n'
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::len",
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
