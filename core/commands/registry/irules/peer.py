# Enriched from F5 iRules reference documentation.
"""peer -- Causes the specified iRule commands to be evaluated under the peer-side context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/peer.html"


@register
class PeerCommand(CommandDef):
    name = "peer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="peer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the specified iRule commands to be evaluated under the peer-side context.",
                synopsis=("peer ANY_CHARS",),
                snippet="Causes the specified iRule commands to be evaluated under the peer-side context.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n  peer { TCP::collect }\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="peer ANY_CHARS",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
