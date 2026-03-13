# Enriched from F5 iRules reference documentation.
"""DIAMETER::result -- Gets or sets the value of the result-code attribute-value pair."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__result.html"


@register
class DiameterResultCommand(CommandDef):
    name = "DIAMETER::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the value of the result-code attribute-value pair.",
                synopsis=("DIAMETER::result (DIAMETER_RESULT_CODE)?",),
                snippet=(
                    "This iRule command gets or sets the value of the result-code (code\n"
                    "268) attribute-value pair."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message with result code [DIAMETER::result]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::result (DIAMETER_RESULT_CODE)?",
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
