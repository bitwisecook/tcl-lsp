# Enriched from F5 iRules reference documentation.
"""WS::enabled -- This command can be used to turn off WebSocket processing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__enabled.html"


_av = make_av(_SOURCE)


@register
class WsEnabledCommand(CommandDef):
    name = "WS::enabled"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::enabled",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to turn off WebSocket processing.",
                synopsis=("WS::enabled ( 'false' )?",),
                snippet=(
                    "WS::enabled\n"
                    "    This can be used to determine whether the Websocket processing is enabled or disabled for a particular connection.\n"
                    "\n"
                    "WS::enabled false\n"
                    "    This can be used to disable the Websocket processing for a particular connection."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    WS::enabled false\n}"),
                return_value="WS::enabled command returns TRUE if Websocket processing is enabled for a particular connection, FALSE otherwise.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::enabled ( 'false' )?",
                    arg_values={
                        0: (_av("false", "WS::enabled false", "WS::enabled ( 'false' )?"),)
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
