# Enriched from F5 iRules reference documentation.
"""LB::mode -- Sets the load balancing mode, overriding the mode set in the pool definition."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__mode.html"


_av = make_av(_SOURCE)


@register
class LbModeCommand(CommandDef):
    name = "LB::mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the load balancing mode, overriding the mode set in the pool definition.",
                synopsis=(
                    "LB::mode (default | rr | roundrobin)",
                    "LB::mode (leastconns | nodeleastconns)",
                    "LB::mode (fastest)",
                    "LB::mode (predictive)",
                ),
                snippet=(
                    "Sets the load balancing mode, overriding the mode set in the pool definition\n"
                    "\n"
                    "LB::mode [default | rr | roundrobin | leastconns |\n"
                    "          fastest | predictive | observed | ratio |\n"
                    "          dynratio | nodeleastconns | noderatio]"
                ),
                source=_SOURCE,
                examples=(
                    "when LB_SELECTED {\n"
                    "    if { $myretry >= 1 } {\n"
                    "        LB::mode rr\n"
                    "        LB::reselect pool $mypool\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::mode (default | rr | roundrobin)",
                    arg_values={
                        0: (
                            _av(
                                "default",
                                "LB::mode default",
                                "LB::mode (default | rr | roundrobin)",
                            ),
                            _av("rr", "LB::mode rr", "LB::mode (default | rr | roundrobin)"),
                            _av(
                                "roundrobin",
                                "LB::mode roundrobin",
                                "LB::mode (default | rr | roundrobin)",
                            ),
                            _av(
                                "leastconns",
                                "LB::mode leastconns",
                                "LB::mode (leastconns | nodeleastconns)",
                            ),
                            _av(
                                "nodeleastconns",
                                "LB::mode nodeleastconns",
                                "LB::mode (leastconns | nodeleastconns)",
                            ),
                            _av("fastest", "LB::mode fastest", "LB::mode (fastest)"),
                            _av("predictive", "LB::mode predictive", "LB::mode (predictive)"),
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
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
