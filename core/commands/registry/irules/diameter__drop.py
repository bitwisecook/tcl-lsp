# Enriched from F5 iRules reference documentation.
"""DIAMETER::drop -- Drops the current message quietly."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__drop.html"


@register
class DiameterDropCommand(CommandDef):
    name = "DIAMETER::drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Drops the current message quietly.",
                synopsis=("DIAMETER::drop",),
                snippet="This iRule command drops the current Diameter message quietly.",
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    if { [DIAMETER::command 275] && [DIAMETER::is_request] } {\n"
                    "        DIAMETER::drop\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::drop",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
