# Enriched from F5 iRules reference documentation.
"""HSL::send -- Sends data via High Speed Logging."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HSL__send.html"


@register
class HslSendCommand(CommandDef):
    name = "HSL::send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HSL::send",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends data via High Speed Logging.",
                synopsis=("HSL::send HANDLE DATA",),
                snippet="Send data via High Speed Logging",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set hsl [HSL::open -proto UDP -pool syslog_server_pool]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HSL::send HANDLE DATA",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
