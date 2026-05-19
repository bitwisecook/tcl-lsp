# Enriched from F5 iRules reference documentation.
"""TCP::payload -- Returns or changes the data collected by TCP::collect."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__payload.html"


@register
class TcpPayloadCommand(CommandDef):
    name = "TCP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or changes the data collected by TCP::collect.",
                synopsis=(
                    "TCP::payload ?<size>?",
                    "TCP::payload replace <offset> <length> <data>",
                    "TCP::payload length",
                ),
                snippet="Returns the accumulated TCP data content, or replaces collected payload with the specified data.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="TCP::payload ?<size>?",
                    arity=Arity(0, 1),
                    pure=True,
                ),
            ),
            subcommands={
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(3, 3),
                    detail="Replace bytes in collected payload.",
                    synopsis="TCP::payload replace <offset> <length> <data>",
                    mutator=True,
                ),
                "length": SubCommand(
                    name="length",
                    arity=Arity(0, 0),
                    detail="Returns the amount of accumulated TCP data in bytes.",
                    synopsis="TCP::payload length",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(0, 4),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
