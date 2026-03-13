# Enriched from F5 iRules reference documentation.
"""SDP::session_id -- Get the SDP session id."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SDP__session_id.html"


@register
class SdpSessionIdCommand(CommandDef):
    name = "SDP::session_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SDP::session_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get the SDP session id.",
                synopsis=("SDP::session_id",),
                snippet=(
                    "This command allows you to get SDP session id for the current\nconnection"
                ),
                source=_SOURCE,
                examples=(
                    'when SIP_RESPONSE {\n    log local0. "SDP SessionID: [SDP::session_id]"\n}'
                ),
                return_value="Returns the value of the SDP session id.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SDP::session_id",
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
