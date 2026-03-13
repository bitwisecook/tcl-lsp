# Enriched from F5 iRules reference documentation.
"""TCP::pacing -- Toggles TCP rate pacing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__pacing.html"


@register
class TcpPacingCommand(CommandDef):
    name = "TCP::pacing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::pacing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP rate pacing.",
                synopsis=("TCP::pacing (BOOL_VALUE)?",),
                snippet="Rate pacing limits the data send rate to the physical limitations of the interface to reduce the chance of queue drops.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side rate pacing to enabled.\n"
                    "    clientside {\n"
                    '        log local0. "Client: pacing [TCP::pacing], enabling"\n'
                    "        TCP::pacing enable\n"
                    "    }\n"
                    "    # Set server-side rate pacing to disabled.\n"
                    "    serverside {\n"
                    '        log local0. "Server: pacing [TCP::pacing], disabling"\n'
                    "        TCP::pacing disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP::pacing returns whether TCP rate pacing is enabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::pacing (BOOL_VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
