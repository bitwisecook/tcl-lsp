# Enriched from F5 iRules reference documentation.
"""TCP::payload -- Returns or changes the data collected by TCP::collect."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
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
                    "TCP::payload (LENGTH | (OFFSET LENGTH))?",
                    "TCP::payload length",
                    "TCP::payload replace OFFSET LENGTH TCP_PAYLOAD",
                ),
                snippet="Returns the accumulated TCP data content, or replaces collected payload with the specified data.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::payload (LENGTH | (OFFSET LENGTH))?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
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
