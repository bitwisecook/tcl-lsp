# Enriched from F5 iRules reference documentation.
"""RTSP::method -- Returns a method/command from the current RTSP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__method.html"


@register
class RtspMethodCommand(CommandDef):
    name = "RTSP::method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::method",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a method/command from the current RTSP request.",
                synopsis=("RTSP::method",),
                snippet=(
                    "Returns the method/command (for example, DESCRIBE, PLAY) from the\n"
                    "current RTSP request."
                ),
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        puts [RTSP::method]\n    }"),
                return_value="Returns a method/command from the current RTSP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::method",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
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
