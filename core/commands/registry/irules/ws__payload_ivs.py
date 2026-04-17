# Enriched from F5 iRules reference documentation.
"""WS::payload_ivs -- Specifies name of the Internal Virtual Server (IVS) that will process websocket payload protocol"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__payload_ivs.html"


@register
class WsPayloadIvsCommand(CommandDef):
    name = "WS::payload_ivs"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::payload_ivs",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies name of the Internal Virtual Server (IVS) that will process websocket payload protocol",
                synopsis=("WS::payload_ivs IVSNAME",),
                snippet=(
                    "WS::payload_ivs <IVS-name>\n"
                    "    Sets the name of Internal Virtual Server (IVS) that will process websocket payload protocol.\n"
                    "    This command takes effect only when payload processing mode of websocket profile is configured to Termination."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_REQUEST {\n"
                    "    switch [WS::request protocol] {\n"
                    "        mqtt {\n"
                    "            WS::payload_processing enable\n"
                    "            WS::payload_ivs /Common/mqtt_ivs\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::payload_ivs IVSNAME",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
