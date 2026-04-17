# Enriched from F5 iRules reference documentation.
"""RTSP::msg_source -- Indicates whether the request or response originated from the client or the server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RTSP__msg_source.html"


@register
class RtspMsgSourceCommand(CommandDef):
    name = "RTSP::msg_source"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RTSP::msg_source",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Indicates whether the request or response originated from the client or the server.",
                synopsis=("RTSP::msg_source",),
                snippet=(
                    "Indicates whether the request or response originated from the client or\n"
                    "the server. This command returns the string client or server."
                ),
                source=_SOURCE,
                examples=("when RTSP_REQUEST {\n        puts [RTSP::msg_source]\n    }"),
                return_value='Returns the string "client" or "server".',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RTSP::msg_source",
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
