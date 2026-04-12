# Enriched from F5 iRules reference documentation.
"""DIAMETER::is_response -- Returns true if it is a DIAMETER response, otherwise, returns false."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__is_response.html"


@register
class DiameterIsResponseCommand(CommandDef):
    name = "DIAMETER::is_response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::is_response",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true if it is a DIAMETER response, otherwise, returns false.",
                synopsis=("DIAMETER::is_response",),
                snippet=(
                    "This iRule command returns true if the current message is a DIAMETER response.\n"
                    "Otherwise, it returns false.\n"
                    "\n"
                    "This command is the exact inverse of DIAMETER::is_request."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    if { [DIAMETER::is_response] } {\n"
                    '        log local0. "Response received"\n'
                    "    }\n"
                    "}"
                ),
                return_value="TRUE or FALSE",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::is_response",
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
