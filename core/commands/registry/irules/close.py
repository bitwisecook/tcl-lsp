# Enriched from F5 iRules reference documentation.
"""close -- Closes an existing sideband connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/close.html"


@register
class CloseCommand(CommandDef):
    name = "close"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="close",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Closes an existing sideband connection.",
                synopsis=("close CONNECTION",),
                snippet="This command closes an existing sideband connection. It is one of several commands that make up the ability to create sideband connections from iRules.",
                source=_SOURCE,
                examples=(
                    "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n"
                    "#   to a local virtual server name sideband_virtual_server\n"
                    "set conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n"
                    "\n"
                    "# Same as above, but use an external host IP:port instead of a virtual server name\n"
                    "set conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n"
                    "\n"
                    "# close the connection\n"
                    "close conn_id"
                ),
                return_value="close <connection> closes an existing connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="close CONNECTION",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
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
