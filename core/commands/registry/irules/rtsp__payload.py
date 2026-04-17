# Enriched from F5 iRules reference documentation.
"""RTSP::payload -- Queries for or replaces content information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__payload.html"


_av = make_av(_SOURCE)


@register
class RtspPayloadCommand(CommandDef):
    name = "RTSP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or replaces content information.",
                synopsis=(
                    "RTSP::payload (LENGTH | length)?",
                    "RTSP::payload replace OFFSET LENGTH RTSP_PAYLOAD",
                ),
                snippet=(
                    "Queries for or replaces content information. With this command, you can\n"
                    "retrieve content, query for content size, or replace a certain amount\n"
                    "of content."
                ),
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        RTSP::collect\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::payload (LENGTH | length)?",
                    arg_values={
                        0: (
                            _av(
                                "length", "RTSP::payload length", "RTSP::payload (LENGTH | length)?"
                            ),
                        )
                    },
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
