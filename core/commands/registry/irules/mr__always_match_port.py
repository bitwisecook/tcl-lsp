# Enriched from F5 iRules reference documentation.
"""MR::always_match_port -- Gets or sets the always_match_port mode for the router."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__always_match_port.html"


@register
class MrAlwaysMatchPortCommand(CommandDef):
    name = "MR::always_match_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::always_match_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the always_match_port mode for the router.",
                synopsis=("MR::always_match_port (BOOLEAN)?",),
                snippet="The MR::always_match_port command sets or resets the always_match_port mode of the current router. If always_match_port mode is enabled (upon completion of CLIENT_ACCEPTED event), the router will only forward messages to existing connections where the remote port matches the remote port of the selected destination. If an existing connection is not found, a new connection will be created. Setting this mode will keep MRF from forwarding messages to incoming connections (since the incoming connection likely uses a ephemeral port as the source port).",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::always_match_port no\n"
                    "            }"
                ),
                return_value="Returns the current value of the always_match_port flag. This will be 'true' or 'false'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::always_match_port (BOOLEAN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
