# Enriched from F5 iRules reference documentation.
"""RTSP::header -- Manages headers in RTSP requests and responses."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__header.html"


_av = make_av(_SOURCE)


@register
class RtspHeaderCommand(CommandDef):
    name = "RTSP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Manages headers in RTSP requests and responses.",
                synopsis=(
                    "RTSP::header (exists | remove | value) HEADER_NAME",
                    "RTSP::header replace HEADER_NAME HEADER_VALUE",
                    "RTSP::header insert (<(HEADER_NAME HEADER_VALUE)+> |",
                ),
                snippet="Manages headers in RTSP requests and responses.",
                source=_SOURCE,
                examples=(
                    'when RTSP_REQUEST {\n        puts [RTSP::header value "x-header"]\n    }'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::header (exists | remove | value) HEADER_NAME",
                    arg_values={
                        0: (
                            _av(
                                "exists",
                                "RTSP::header exists",
                                "RTSP::header (exists | remove | value) HEADER_NAME",
                            ),
                            _av(
                                "remove",
                                "RTSP::header remove",
                                "RTSP::header (exists | remove | value) HEADER_NAME",
                            ),
                            _av(
                                "value",
                                "RTSP::header value",
                                "RTSP::header (exists | remove | value) HEADER_NAME",
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
