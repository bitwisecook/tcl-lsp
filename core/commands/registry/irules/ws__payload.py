# Enriched from F5 iRules reference documentation.
"""WS::payload -- Queries for or manipulates Websocket frame payload information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__payload.html"


@register
class WsPayloadCommand(CommandDef):
    name = "WS::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or manipulates Websocket frame payload information.",
                synopsis=(
                    "WS::payload (LENGTH | (OFFSET LENGTH))?",
                    "WS::payload length",
                    "WS::payload replace OFFSET LENGTH STRING",
                ),
                snippet=(
                    "WS::payload <length>\n"
                    "    Returns the content that the WS::collect command has collected thus far, up to the number of bytes specified. If you do not specify a size, the system returns the entire collected content.\n"
                    "\n"
                    "WS::payload <offset> <length>\n"
                    "    Returns the content that the WS::collect command has collected thus far from the specified offset, up to the number of bytes specified.\n"
                    "\n"
                    "WS::payload length\n"
                    "    Returns the size of the content that has been collected thus far, in bytes."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_CLIENT_FRAME {\n    WS::collect frame 1000\n    set clen 1000\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::payload (LENGTH | (OFFSET LENGTH))?",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
