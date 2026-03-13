# Enriched from F5 iRules reference documentation.
"""DNS::drop -- Drops the current DNS packet after the execution of the event."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__drop.html"


@register
class DnsDropCommand(CommandDef):
    name = "DNS::drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Drops the current DNS packet after the execution of the event.",
                synopsis=("DNS::drop",),
                snippet=(
                    "This iRules command drops the current DNS packet after the execution of\n"
                    "the event.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "quests from a specific IP\n"
                    "            when DNS_REQUEST {\n"
                    '                if { [IP::client_addr] equals "192.168.1.245" } {\n'
                    "                    DNS::drop\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::drop",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
