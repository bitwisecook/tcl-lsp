# Enriched from F5 iRules reference documentation.
"""NSH::chain -- Sets the Chain for flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__chain.html"


@register
class NshChainCommand(CommandDef):
    name = "NSH::chain"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::chain",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the Chain for flow.",
                synopsis=("NSH::chain DIRECTION CHAIN_NAME",),
                snippet="Set: chain for NSH.",
                source=_SOURCE,
                examples=(
                    "ntext for NSH.\n"
                    "            when CLIENT_ACCEPTED {\n"
                    "                NSH::chain clientside_ingress test1\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::chain DIRECTION CHAIN_NAME",
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
