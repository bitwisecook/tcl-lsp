# Enriched from F5 iRules reference documentation.
"""TCP::enhanced_loss_recovery -- Toggles enhanced loss recovery."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__enhanced_loss_recovery.html"


@register
class TcpEnhancedLossRecoveryCommand(CommandDef):
    name = "TCP::enhanced_loss_recovery"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::enhanced_loss_recovery",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles enhanced loss recovery.",
                synopsis=("TCP::enhanced_loss_recovery BOOL_VALUE",),
                snippet="Enables or disables enhanced loss recovery which recovers from random packet loss more effectively.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    TCP::enhanced_loss_recovery enable\n}"),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::enhanced_loss_recovery BOOL_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
