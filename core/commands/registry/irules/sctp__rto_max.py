# Enriched from F5 iRules reference documentation.
"""SCTP::rto_max -- Returns the maximum value of SCTP retransmission timeout."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__rto_max.html"


_av = make_av(_SOURCE)


@register
class SctpRtoMaxCommand(CommandDef):
    name = "SCTP::rto_max"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::rto_max",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the maximum value of SCTP retransmission timeout.",
                synopsis=("SCTP::rto_max (clientside | serverside)?",),
                snippet="Returns the maximum value of SCTP retranmission timeout. Can specify the value on clientside or serverside.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '        log local0.info "SCTP retransmission timeout maximum value is [SCTP::rto_max]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::rto_max (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SCTP::rto_max clientside",
                                "SCTP::rto_max (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SCTP::rto_max serverside",
                                "SCTP::rto_max (clientside | serverside)?",
                            ),
                        )
                    },
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
