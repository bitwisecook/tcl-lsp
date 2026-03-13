# Enriched from F5 iRules reference documentation.
"""DIAMETER::retransmit -- Triggers the request associated to the current answer message for retransmission."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retransmit.html"


_av = make_av(_SOURCE)


@register
class DiameterRetransmitCommand(CommandDef):
    name = "DIAMETER::retransmit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retransmit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Triggers the request associated to the current answer message for retransmission.",
                synopsis=("DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",),
                snippet=(
                    "This iRule command triggers the request in the retransmission queue\n"
                    "that is associated with the current answer message for\n"
                    "retransmission. This command will fail the current message is a\n"
                    "request or if there is not an associated request message in the\n"
                    "retransmission queue."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_EGRESS {\n"
                    "    if { [DIAMETER::is_response] && ![DIAMETER::is_retransmission] } {\n"
                    '        log local0. "reason [DIAMETER::retransmission_reason]"\n'
                    "        DIAMETER::retransmit\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                    arg_values={
                        0: (
                            _av(
                                "disabled",
                                "DIAMETER::retransmit disabled",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                            _av(
                                "busy",
                                "DIAMETER::retransmit busy",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                            _av(
                                "unable",
                                "DIAMETER::retransmit unable",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                            _av(
                                "retransmit",
                                "DIAMETER::retransmit retransmit",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
