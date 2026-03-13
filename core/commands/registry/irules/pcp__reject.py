# Enriched from F5 iRules reference documentation.
"""PCP::reject -- Provides the ability to cause a PCP reqeust to fail based on processing in the iRule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PCP__reject.html"


@register
class PcpRejectCommand(CommandDef):
    name = "PCP::reject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PCP::reject",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides the ability to cause a PCP reqeust to fail based on processing in the iRule.",
                synopsis=("PCP::reject PCP_RESULT_CODE",),
                snippet=(
                    "This command provides the ability to cause a PCP (Port Control\n"
                    "Protocol) reqeust to fail based on processing in the iRule. If the\n"
                    "reject command is issued, the PCP request is rejected with the\n"
                    "specified result code and no other action is taken by the PCP proxy."
                ),
                source=_SOURCE,
                examples=(
                    "when PCP_REQUEST {\n"
                    '     if {[PCP::request opcode] == "map" &&\n'
                    "             [PCP::request internal-port] == 22 } {\n"
                    '         log "Rejecting PCP request to map SSH"\n'
                    "         PCP::reject 1\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PCP::reject PCP_RESULT_CODE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"PCP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
