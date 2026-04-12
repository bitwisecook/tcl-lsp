# Enriched from F5 iRules reference documentation.
"""MR::connection_mode -- Returns the connection mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__connection_mode.html"


@register
class MrConnectionModeCommand(CommandDef):
    name = "MR::connection_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::connection_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the connection mode.",
                synopsis=("MR::connection_mode",),
                snippet=(
                    "returns the connection mode of the current connection and the number of\n"
                    "as configured in the peer object used to create the connection. Valid\n"
                    'connection modes as "per-peer", "per-blade", "per-tmm" or "per-client".\n'
                    'For incoming connections, it will return "per-peer".'
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "[MR::connection_instance] [MR::connection_mode]"\n'
                    "}"
                ),
                return_value="returns the connection mode",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::connection_mode",
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
