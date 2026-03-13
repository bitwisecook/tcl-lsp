# Enriched from F5 iRules reference documentation.
"""UDP::drop -- Drops the current UDP packet without removing the flow from the connection table."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__drop.html"


@register
class UdpDropCommand(CommandDef):
    name = "UDP::drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Drops the current UDP packet without removing the flow from the connection table.",
                synopsis=("UDP::drop",),
                snippet=(
                    "Drops the current UDP packet without removing the flow from the\n"
                    "connection table"
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_DATA {\n"
                    '    if { [UDP::payload contains "badstring"] }{\n'
                    "        UDP::drop\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::drop",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="udp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
