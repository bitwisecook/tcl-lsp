# Enriched from F5 iRules reference documentation.
"""TCP::release -- Release data gathered by TCP::collect to the upper layer."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__release.html"


@register
class TcpReleaseCommand(CommandDef):
    name = "TCP::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Release data gathered by TCP::collect to the upper layer.",
                synopsis=("TCP::release (LENGTH)?",),
                snippet=(
                    "Causes TCP to release and flush collected data, and allow other\n"
                    "protocol layers to resume processing the connection.\n"
                    "\n"
                    "Returns the number of bytes actually released. If specified, up to length bytes are released; the return value will tell you how many bytes actually were."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect 15\n}"),
                return_value="The number of bytes released.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::release (LENGTH)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
