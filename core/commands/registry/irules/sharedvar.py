# Enriched from F5 iRules reference documentation.
"""sharedvar -- Allows a variable to be accessed in both sides of a VIP-targetting-VIP."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/sharedvar.html"


@register
class SharedvarCommand(CommandDef):
    name = "sharedvar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="sharedvar",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows a variable to be accessed in both sides of a VIP-targetting-VIP.",
                synopsis=("sharedvar VARIABLE",),
                snippet="Allows a variable to be accessed in both sides of a VIP-targetting-VIP",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    'log local0. "vip1 @ response: private: $private"\n'
                    'log local0. "vip1 @ response: public: $public"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sharedvar VARIABLE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
