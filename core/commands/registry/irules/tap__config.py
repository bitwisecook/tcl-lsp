# Enriched from F5 iRules reference documentation.
"""TAP::config -- Returns the current value of the specified setting in an assigned TAP Application."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TAP__config.html"


@register
class TapConfigCommand(CommandDef):
    name = "TAP::config"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TAP::config",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current value of the specified setting in an assigned TAP Application.",
                synopsis=("TAP::config APPLICATION ENTITY",),
                snippet="Returns the current value of the specified setting in an assigned TAP Application.",
                source=_SOURCE,
                return_value="Returns the current value of the specified setting in an assigned TAP Application.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TAP::config APPLICATION ENTITY",
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
