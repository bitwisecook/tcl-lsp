# Enriched from F5 iRules reference documentation.
"""FLOW::idle_timeout -- Sets/Gets the idle timeout on the flow"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__idle_timeout.html"


@register
class FlowIdleTimeoutCommand(CommandDef):
    name = "FLOW::idle_timeout"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::idle_timeout",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets/Gets the idle timeout on the flow",
                synopsis=("FLOW::idle_timeout (ANY_CHARS) (NONNEGATIVE_INTEGER)?",),
                snippet="Sets/Gets the idle timeout on the flow.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set cf [FLOW::this]\n"
                    "\n"
                    "    #Get flow idletimeout\n"
                    '    log local0. "Idle timeout: [FLOW::idle_timeout $cf]"\n'
                    "\n"
                    "    #Set flow idletimeout\n"
                    "    FLOW::idle_timeout $cf 100\n"
                    "\n"
                    "    unset cf\n"
                    "}"
                ),
                return_value="Set operation: Nothing is returned Get operation: Idle timeout set on the flow as number string. On error an exception is thrown with a message indicating the cause of failure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::idle_timeout (ANY_CHARS) (NONNEGATIVE_INTEGER)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FLOW"}),
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_DATA",
                        "LB_SELECTED",
                        "SA_PICKED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                ),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FLOW_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
