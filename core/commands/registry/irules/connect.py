# Enriched from F5 iRules reference documentation.
"""connect -- Establishes a sideband connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/connect.html"


@register
class ConnectCommand(CommandDef):
    name = "connect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="connect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Establishes a sideband connection.",
                synopsis=(
                    "connect info (",
                    "connect ((",
                ),
                snippet="This command establishes a sideband connection. It is one of several commands that make up the ability to use sideband connections from iRules.",
                source=_SOURCE,
                examples=(
                    "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n"
                    "#   to a local virtual server name sideband_virtual_server\n"
                    "set conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n"
                    "\n"
                    "# Same as above, but use an external host IP:port instead of a virtual server name\n"
                    "set conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n"
                    "\n"
                    "\n"
                    "Example with more complete error handling:"
                ),
                return_value="This command opens a sideband connection to the specified destination.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="connect info (",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_network_sink_args=(),
            event_requires=EventRequires(flow=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
