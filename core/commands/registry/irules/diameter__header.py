# Enriched from F5 iRules reference documentation.
"""DIAMETER::header -- Gets or sets the DIAMETER header fields."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__header.html"


@register
class DiameterHeaderCommand(CommandDef):
    name = "DIAMETER::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the DIAMETER header fields.",
                synopsis=("DIAMETER::header (",),
                snippet="This iRule command is used to get and set header fields in the current DIAMETER message.",
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    if { [DIAMETER::header tflag] } {\n"
                    '        log local0. "Received a potentially retransmitted Diameter message"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::header (",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
