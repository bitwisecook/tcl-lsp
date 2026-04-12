# Enriched from F5 iRules reference documentation.
"""PCP::request -- Provides access to the data sent in a PCP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PCP__request.html"


@register
class PcpRequestCommand(CommandDef):
    name = "PCP::request"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PCP::request",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides access to the data sent in a PCP request.",
                synopsis=("PCP::request (opcode |",),
                snippet=(
                    "This command provides access to the data sent in a PCP (Port Control\n"
                    "Protocol) request. Access to this data is read-only, and the data in\n"
                    "the PCP request cannot be modified via the PCP::request command."
                ),
                source=_SOURCE,
                examples=(
                    "when PCP_REQUEST {\n"
                    '     if {[PCP::request opcode] == "map" && [PCP::request client-addr] == "192.168.1.1" } {\n'
                    '         log "Received PCP map request for port [PCP::request internal-port] from 192.168.1.1"\n'
                    "     }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PCP::request (opcode |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"PCP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
