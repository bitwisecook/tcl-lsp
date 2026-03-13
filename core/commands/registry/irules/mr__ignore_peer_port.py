# Enriched from F5 iRules reference documentation.
"""MR::ignore_peer_port -- Gets or sets the ignore_peer_port mode for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__ignore_peer_port.html"


@register
class MrIgnorePeerPortCommand(CommandDef):
    name = "MR::ignore_peer_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::ignore_peer_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the ignore_peer_port mode for the current connection.",
                synopsis=("MR::ignore_peer_port (BOOLEAN)?",),
                snippet="The MR::ignore_peer_port command sets or resets the ignore_peer_port mode of the current connection. If ignore_peer_port mode is enabled, the remote port of the connection will be ignored when determining if the connection is usable for forwarding a message to a peer. For example, if a peer at IP 10.1.2.3 connects using a ephemeral port of 12345 and ignore_peer_port is enabled, a message routed to IP 10.1.2.3 port 2345 can be forwarded using this connection since the port will be ignored.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::ignore_peer_port yes\n"
                    "            }"
                ),
                return_value="Returns the current value of the ignore_peer_port flag. This will be 'true' or 'false'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::ignore_peer_port (BOOLEAN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
