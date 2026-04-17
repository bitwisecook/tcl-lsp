# Enriched from F5 iRules reference documentation.
"""RTSP::uri -- Returns the complete URI of the RTSP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__uri.html"


@register
class RtspUriCommand(CommandDef):
    name = "RTSP::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::uri",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the complete URI of the RTSP request.",
                synopsis=("RTSP::uri (URI_STRING)?",),
                snippet="Returns the complete URI of the RTSP request.",
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        puts [RTSP::uri]\n    }"),
                return_value="Returns the complete URI of the RTSP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::uri (URI_STRING)?",
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
