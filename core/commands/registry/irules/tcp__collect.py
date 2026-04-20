# Enriched from F5 iRules reference documentation.
"""TCP::collect -- Collects the specified amount of data for delivery to the iRule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__collect.html"


@register
class TcpCollectCommand(CommandDef):
    name = "TCP::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collects the specified amount of data for delivery to the iRule.",
                synopsis=("TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?",),
                snippet=(
                    "Collects the specified amount of data before triggering a CLIENT_DATA or SERVER_DATA event. This data is not delivered to the upper layer until the event fires.\n"
                    "\n"
                    "After collecting the data in a clientside event, the CLIENT_DATA\n"
                    "event will be triggered. When collecting the data in a serverside\n"
                    "event, the SERVER_DATA event will be triggered.\n"
                    "\n"
                    "It is important to note that, when an explicit length is not specified,\n"
                    "the semantics of TCP::collect and TCP::release are different than\n"
                    "those of the HTTP::collect and HTTP::release commands.\n"
                    "\n"
                    "**Caution**: If multiple iRules call `TCP::collect` on the same\n"
                    "connection, only the last call takes effect (last-execution-wins).\n"
                    "Use a marker variable to coordinate:\n"
                    "```tcl\n"
                    "set tcp_collect 0\n"
                    "if { !$tcp_collect } {\n"
                    "    set tcp_collect 1\n"
                    "    TCP::collect 43\n"
                    "}\n"
                    "```"
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # First collect 50000 bytes before passing data to the client.\n"
                    "    TCP::collect 50000\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
