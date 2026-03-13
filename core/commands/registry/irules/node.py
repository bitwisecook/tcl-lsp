"""node -- Route traffic directly to a specific node."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/node.html"


@register
class NodeCommand(CommandDef):
    name = "node"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="node",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Route traffic directly to a specific node.",
                synopsis=("node ip_addr ?service_port?",),
                snippet="Bypasses pool selection and targets an explicit backend endpoint.",
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="node ip_addr ?service_port?"),),
            validation=ValidationSpec(arity=Arity(1, 2)),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            cse_candidate=True,
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NODE_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
