# Enriched from F5 iRules reference documentation.
"""MR::flow_id -- Returns a unique identifier for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__flow_id.html"


@register
class MrFlowIdCommand(CommandDef):
    name = "MR::flow_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::flow_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a unique identifier for the current connection.",
                synopsis=("MR::flow_id",),
                snippet="Returns a unique identifier for the current connection. This identifier can be used to generate the lasthop and nexthop of a message.",
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    set orig_flowid [MR::flow_id]\n"
                    "    MR::store orig_flowid\n"
                    "}"
                ),
                return_value="Returns a unique identifier for the current connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::flow_id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
