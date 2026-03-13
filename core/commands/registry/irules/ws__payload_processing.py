# Enriched from F5 iRules reference documentation.
"""WS::payload_processing -- Enables or disables processing of WebSocket payload via payload protocol filter"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__payload_processing.html"


_av = make_av(_SOURCE)


@register
class WsPayloadProcessingCommand(CommandDef):
    name = "WS::payload_processing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::payload_processing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables or disables processing of WebSocket payload via payload protocol filter",
                synopsis=("WS::payload_processing ('enable' | 'disable')",),
                snippet=(
                    "WS::payload_processing enable\n"
                    "    Enables processing of WebSocket Payload via payload protocol filter\n"
                    "WS::payload_processing disable\n"
                    "    Disables processing of WebSocket Payload via payload protocol filter"
                ),
                source=_SOURCE,
                examples=(
                    "when WS_REQUEST {\n"
                    "    switch [WS::request protocol] {\n"
                    "        mqtt {\n"
                    "            WS::payload_processing enable\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::payload_processing ('enable' | 'disable')",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "WS::payload_processing enable",
                                "WS::payload_processing ('enable' | 'disable')",
                            ),
                            _av(
                                "disable",
                                "WS::payload_processing disable",
                                "WS::payload_processing ('enable' | 'disable')",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
