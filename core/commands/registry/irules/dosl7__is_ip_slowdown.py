# Enriched from F5 iRules reference documentation.
"""DOSL7::is_ip_slowdown -- Returns TRUE if source IP exists in greylist table"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__is_ip_slowdown.html"


@register
class Dosl7IsIpSlowdownCommand(CommandDef):
    name = "DOSL7::is_ip_slowdown"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::is_ip_slowdown",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns TRUE if source IP exists in greylist table",
                synopsis=("DOSL7::is_ip_slowdown",),
                snippet="Returns TRUE if source IP exists in greylist table",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::is_ip_slowdown",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DOSL7_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
