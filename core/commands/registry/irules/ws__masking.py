# Enriched from F5 iRules reference documentation.
"""WS::masking -- This command determines the behavior of Websocket processing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__masking.html"


_av = make_av(_SOURCE)


@register
class WsMaskingCommand(CommandDef):
    name = "WS::masking"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::masking",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command determines the behavior of Websocket processing.",
                synopsis=("WS::masking ( 'preserve' | 'remask' )",),
                snippet=(
                    "WS::masking preserve\n"
                    "    The WebSockets module will not unmask the payload. Data received from the end-points will be sent untouched to other modules for further processing.\n"
                    "\n"
                    "WS::masking remask\n"
                    "    The data received from the end-points is unmasked and sent to other modules for further processing. The client-to-server frame's payload is then masked with the specified mask before sending data out on the wire again."
                ),
                source=_SOURCE,
                examples=("when WS_REQUEST {\n    WS::masking preserve\n    WS::masking remask\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::masking ( 'preserve' | 'remask' )",
                    arg_values={
                        0: (
                            _av(
                                "preserve",
                                "WS::masking preserve",
                                "WS::masking ( 'preserve' | 'remask' )",
                            ),
                            _av(
                                "remask",
                                "WS::masking remask",
                                "WS::masking ( 'preserve' | 'remask' )",
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
