# Enriched from F5 iRules reference documentation.
"""SCTP::mss -- Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__mss.html"


@register
class SctpMssCommand(CommandDef):
    name = "SCTP::mss"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::mss",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection.",
                synopsis=("SCTP::mss",),
                snippet="Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "        SCTP::collect\n"
                    '        log local0.info "Sctp local port is [SCTP::local_port]"\n'
                    '        log local0.info "Sctp client port is [SCTP::client_port]"\n'
                    '        log local0.info "Sctp mss is [SCTP::mss]"\n'
                    '        log local0.info "sctp ppi is [SCTP::ppi]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::mss",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
