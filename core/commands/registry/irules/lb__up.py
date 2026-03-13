# Enriched from F5 iRules reference documentation.
"""LB::up -- Sets the status of a node or pool member as being up."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__up.html"


_av = make_av(_SOURCE)


@register
class LbUpCommand(CommandDef):
    name = "LB::up"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::up",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the status of a node or pool member as being up.",
                synopsis=("LB::up ((node IP_ADDR) |",),
                snippet="Sets the status of the specified node or pool member as being up. If you specify no arguments, the status of the currently-selected node is modified.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::up ((node IP_ADDR) |",
                    arg_values={0: (_av("node", "LB::up node", "LB::up ((node IP_ADDR) |"),)},
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
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
