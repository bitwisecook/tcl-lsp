# Enriched from F5 iRules reference documentation.
"""DIAMETER::is_request -- Returns true if the current message is a DIAMETER request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__is_request.html"


@register
class DiameterIsRequestCommand(CommandDef):
    name = "DIAMETER::is_request"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::is_request",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true if the current message is a DIAMETER request.",
                synopsis=("DIAMETER::is_request",),
                snippet=(
                    "This iRule command returns true if the current message is a DIAMETER request.\n"
                    "Otherwise, it returns false."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    if { [DIAMETER::is_request] } {\n"
                    '        log local0. "Request received"\n'
                    "    }\n"
                    "}"
                ),
                return_value="TRUE or FALSE",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::is_request",
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
