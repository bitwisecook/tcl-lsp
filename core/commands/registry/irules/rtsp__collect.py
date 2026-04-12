# Enriched from F5 iRules reference documentation.
"""RTSP::collect -- Collects the amount of data that you specify."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__collect.html"


@register
class RtspCollectCommand(CommandDef):
    name = "RTSP::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collects the amount of data that you specify.",
                synopsis=("RTSP::collect (LENGTH)?",),
                snippet="Collects the amount of data that you specify.",
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        RTSP::collect 10\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::collect (LENGTH)?",
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
