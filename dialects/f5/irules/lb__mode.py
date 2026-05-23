# Enriched from F5 iRules reference documentation.
"""LB::mode -- Sets the load balancing mode, overriding the mode set in the pool definition."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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
                    synopsis="LB::mode <mode>",
                    arg_values={
                        0: (
                            _av("default", "Use default pool LB mode.", "LB::mode default"),
                            _av("rr", "Round robin.", "LB::mode rr"),
                            _av("roundrobin", "Round robin (alias).", "LB::mode roundrobin"),
                            _av("leastconns", "Least connections.", "LB::mode leastconns"),
                            _av(
                                "nodeleastconns",
                                "Node least connections.",
                                "LB::mode nodeleastconns",
                            ),
                            _av("fastest", "Fastest response.", "LB::mode fastest"),
                            _av("predictive", "Predictive.", "LB::mode predictive"),
                            _av("observed", "Observed.", "LB::mode observed"),
                            _av("ratio", "Ratio.", "LB::mode ratio"),
                            _av("noderatio", "Node ratio.", "LB::mode noderatio"),
                            _av("dynratio", "Dynamic ratio.", "LB::mode dynratio"),
                            _av("dynratiombr", "Dynamic ratio MBR.", "LB::mode dynratiombr"),
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
