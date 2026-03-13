# Enriched from F5 iRules reference documentation.
"""active_members -- Returns the number or list of active members in the specified pool."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/active_members.html"


@register
class ActiveMembersCommand(CommandDef):
    name = "active_members"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="active_members",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number or list of active members in the specified pool.",
                synopsis=("active_members ('-list')? POOL_OBJ",),
                snippet="Returns the number or list of active members in the specified pool.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if { [active_members http_pool] >= 2 } {\n"
                    "        pool http_pool\n"
                    "    }\n"
                    "}"
                ),
                return_value="active_members <pool_name> Returns the number of active members in the specified pool.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="active_members ('-list')? POOL_OBJ",
                    options=(OptionSpec(name="-list", detail="Option -list.", takes_value=True),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"DNS"}),
                also_in=frozenset({"LB_FAILED", "LB_SELECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
