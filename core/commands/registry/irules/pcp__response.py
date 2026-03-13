# Enriched from F5 iRules reference documentation.
"""PCP::response -- Provides access to the data in a PCP response packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PCP__response.html"


@register
class PcpResponseCommand(CommandDef):
    name = "PCP::response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PCP::response",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides access to the data in a PCP response packet.",
                synopsis=("PCP::response (opcode |",),
                snippet=(
                    "This command provides access to the data in a PCP (Port Control\n"
                    "Protocol) response packet. Access to this data is read-only, and the\n"
                    "data in the PCP response cannot be modified via the PCP::response\n"
                    "command."
                ),
                source=_SOURCE,
                examples=(
                    "when PCP_RESPONSE {\n"
                    '    if {[PCP::response opcode] == "map" && [PCP::response result] != 0] } {\n'
                    '        log "PCP map request from\\\n'
                    "              [PCP::response client-addr]:[PCP::response internal-port]\\\n"
                    '              failed with a result of [PCP::response result]"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PCP::response (opcode |",
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
