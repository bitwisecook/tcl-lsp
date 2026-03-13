# Enriched from F5 iRules reference documentation.
"""CONNECTOR::remap -- Set client/server IP/Port from connector."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CONNECTOR__remap.html"


@register
class ConnectorRemapCommand(CommandDef):
    name = "CONNECTOR::remap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CONNECTOR::remap",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set client/server IP/Port from connector.",
                synopsis=(
                    "CONNECTOR::remap server_addr IP_ADDR",
                    "CONNECTOR::remap client_addr IP_ADDR",
                    "CONNECTOR::remap client_port PORT",
                    "CONNECTOR::remap server_port PORT",
                ),
                snippet=(
                    "CONNECTOR::remap client_addr\n"
                    "    Set the client IP address from connector profile.\n"
                    "CONNECTOR::remap server_addr\n"
                    "    Set the server IP address from connector profile.\n"
                    "CONNECTOR::remap client_port\n"
                    "    Set the client port from connector profile.\n"
                    "CONNECTOR::remap server_port\n"
                    "    Set the server port from connector profile."
                ),
                source=_SOURCE,
                examples=(
                    "when CONNECTOR_OPEN {\n"
                    '                if {([CONNECTOR::profile] eq "/Common/connector_profile_1")} {\n'
                    "                    CONNECTOR::remap client_addr 10.10.10.2\n"
                    '                    log local0. "Remap client IP address from connector to 10.10.10.2"\n'
                    "                    CONNECTOR::remap client_port 333\n"
                    '                    log local0. "Remap client port from connector to 333"\n'
                    "                    CONNECTOR::remap server_addr 20.20.20.2"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CONNECTOR::remap server_addr IP_ADDR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
