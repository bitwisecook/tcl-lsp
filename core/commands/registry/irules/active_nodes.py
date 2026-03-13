# Enriched from F5 iRules reference documentation.
"""active_nodes -- Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .active_members import ActiveMembersCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/active_nodes.html"


@register
class ActiveNodesCommand(CommandDef):
    name = "active_nodes"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="active_nodes",
            deprecated_replacement=ActiveMembersCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility).",
                synopsis=("active_nodes ('-list')? POOL_OBJ",),
                snippet="Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility).",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0. "There are [active_nodes http_pool] active nodes in the pool."\n'
                    "}"
                ),
                return_value="active_nodes <pool name> Returns the number of active members of the specified pool (for BIG-IP version 4.X compatibility).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="active_nodes ('-list')? POOL_OBJ",
                    options=(OptionSpec(name="-list", detail="Option -list.", takes_value=True),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
