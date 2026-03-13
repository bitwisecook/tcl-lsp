# Enriched from F5 iRules reference documentation.
"""RTSP::status -- Returns the HTTP style status code from the current RTSP response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__status.html"


@register
class RtspStatusCommand(CommandDef):
    name = "RTSP::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the HTTP style status code from the current RTSP response.",
                synopsis=("RTSP::status",),
                snippet=(
                    "Returns the HTTP style status code (for example, 200 or 401) from the\n"
                    "current RTSP response."
                ),
                source=_SOURCE,
                examples=("when RTSP_RESPONSE {\n        puts [RTSP::status]\n    }"),
                return_value="Returns the HTTP style status code from the current RTSP response.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::status",
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
