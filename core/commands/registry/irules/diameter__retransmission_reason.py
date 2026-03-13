# Enriched from F5 iRules reference documentation.
"""DIAMETER::retransmission_reason -- Returns the reason for retransmitting the current retransmitted request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission_reason.html"


@register
class DiameterRetransmissionReasonCommand(CommandDef):
    name = "DIAMETER::retransmission_reason"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retransmission_reason",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the reason for retransmitting the current retransmitted request.",
                synopsis=("DIAMETER::retransmission_reason",),
                snippet=(
                    "This iRule command returns the reason the current request was retransmitted.\n"
                    "Otherwise, it returns 'none'."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    if { [DIAMETER::is_retransmission] } {\n"
                    '        log local0. "reason [DIAMETER::retransmission_reason]"\n'
                    "        DIAMETER::persist reset\n"
                    "        MR::message route pool /Common/alt_pool\n"
                    "    }\n"
                    "}"
                ),
                return_value="'none', 'error_code', 'timeout' or 'irule'",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retransmission_reason",
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
