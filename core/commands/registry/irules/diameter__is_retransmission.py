# Enriched from F5 iRules reference documentation.
"""DIAMETER::is_retransmission -- Returns true if it is a retransmitted request, otherwise, returns false."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__is_retransmission.html"


@register
class DiameterIsRetransmissionCommand(CommandDef):
    name = "DIAMETER::is_retransmission"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::is_retransmission",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true if it is a retransmitted request, otherwise, returns false.",
                synopsis=("DIAMETER::is_retransmission",),
                snippet=(
                    "This iRule command returns true if the current message is a retransmitted request.\n"
                    "Otherwise, it returns false."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    if { [DIAMETER::is_retransmission] } {\n"
                    "        DIAMETER::persist reset\n"
                    "        MR::message route pool /Common/alt_pool\n"
                    "    }\n"
                    "}"
                ),
                return_value="TRUE or FALSE",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::is_retransmission",
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
