# Enriched from F5 iRules reference documentation.
"""NSH::context -- Sets/Get the Context header for NSH."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__context.html"


@register
class NshContextCommand(CommandDef):
    name = "NSH::context"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::context",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets/Get the Context header for NSH.",
                synopsis=("NSH::context NSH_CONTEXT_IDX DIRECTION (CONTEXT)?",),
                snippet=(
                    "Set: context for NSH.\n"
                    "            Get(NSH_CONTEXT_IDX and DIRECTION as the only parameter): context from NSH."
                ),
                source=_SOURCE,
                examples=(
                    "ntext for NSH.\n"
                    "            when CLIENT_ACCEPTED {\n"
                    "                NSH::context 1 serverside_egress 1111\n"
                    "                set myctx1 [NSH::context 1 serverside_egress]\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::context NSH_CONTEXT_IDX DIRECTION (CONTEXT)?",
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
