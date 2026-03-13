# Enriched from F5 iRules reference documentation.
"""MR::store -- Stores a tcl variable with the mr_message object."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__store.html"


@register
class MrStoreCommand(CommandDef):
    name = "MR::store"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::store",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Stores a tcl variable with the mr_message object.",
                synopsis=("MR::store (VAR)*",),
                snippet="The MR::store command stores one or more named Tcl variables with the message so that they are available on egress even if stored on ingress. If no name is provided, it stores all local variables in the current message context. Storing variables does not affect the content of the message.",
                source=_SOURCE,
                examples=("when MR_EGRESS {\n    MR::restore client_addr\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::store (VAR)*",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
