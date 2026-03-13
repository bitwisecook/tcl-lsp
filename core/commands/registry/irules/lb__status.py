# Enriched from F5 iRules reference documentation.
"""LB::status -- Returns the status of a node address or pool member."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__status.html"


@register
class LbStatusCommand(CommandDef):
    name = "LB::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the status of a node address or pool member.",
                synopsis=(
                    "LB::status (LB_STATUS)?",
                    "LB::status node IP_ADDR (LB_STATUS)?",
                    "LB::status pool POOL_OBJ member IP_ADDR PORT (LB_STATUS)?",
                ),
                snippet=(
                    "Returns the status of a node address or pool member. Possible status values are up, down, session_enabled, and session_disabled. If you supply no arguments, returns the status of the currently-selected pool member.\n"
                    "Syntax:\n"
                    "    LB::status\n"
                    "    LB::status node <address>\n"
                    "    LB::status pool <pool name> member <IP address> <port>\n"
                    "    LB::status <up | down | session_enabled | session_disabled>\n"
                    "    LB::status node <address> <up | down | session_enabled | session_disabled>\n"
                    "    LB::status pool <pool name> member <address> <port> <up | down | session_enabled | session_disabled>"
                ),
                source=_SOURCE,
                examples=(
                    "when LB_FAILED {\n"
                    '    if { [LB::status pool $poolname member $ip $port] eq "down" } {\n'
                    '        log "Server $ip $port down!"\n'
                    "    }\n"
                    "}"
                ),
                return_value="LB::status Returns the status of the currently-selected node (after LB_SELECTED event only). Possible values are: up | down | session_enabled | session_disabled",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::status (LB_STATUS)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
