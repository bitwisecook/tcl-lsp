"""pool -- Select a load-balancing pool for the current flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/pool.html"


@register
class PoolCommand(CommandDef):
    name = "pool"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pool",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Select a load-balancing pool for the current flow.",
                synopsis=("pool pool_name ?member_addr member_port?",),
                snippet="Can direct traffic to a pool, optionally pinning to a specific member.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT, synopsis="pool pool_name ?member_addr member_port?"
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
            event_requires=EventRequires(client_side=True),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
