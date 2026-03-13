# Enriched from F5 iRules reference documentation.
"""RTSP::release -- Releases the collected data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__release.html"


@register
class RtspReleaseCommand(CommandDef):
    name = "RTSP::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the collected data.",
                synopsis=("RTSP::release",),
                snippet=(
                    "Releases the collected data. Unless a subsequent RTSP::collect command\n"
                    "was issued, there is no need to use the RTSP::release command inside of\n"
                    "the RTSP_REQUEST_DATA and RTSP_RESPONSE_DATA events, since in these\n"
                    "cases, the data is implicitly released."
                ),
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        RTSP::collect\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
