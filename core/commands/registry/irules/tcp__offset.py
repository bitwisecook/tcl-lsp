# Enriched from F5 iRules reference documentation.
"""TCP::offset -- Returns the number of bytes held in memory via TCP::collect."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__offset.html"


@register
class TcpOffsetCommand(CommandDef):
    name = "TCP::offset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::offset",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number of bytes held in memory via TCP::collect.",
                synopsis=("TCP::offset",),
                snippet=(
                    "Returns the number of bytes currently held in memory via\n"
                    "TCP::collect. This data is available via TCP::payload."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect\n}"),
                return_value="The number of bytes collected.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::offset",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
