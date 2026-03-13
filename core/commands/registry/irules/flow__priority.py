# Enriched from F5 iRules reference documentation.
"""FLOW::priority -- Set/Get flow's internal packet priority."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__priority.html"


_av = make_av(_SOURCE)


@register
class FlowPriorityCommand(CommandDef):
    name = "FLOW::priority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::priority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set/Get flow's internal packet priority.",
                synopsis=(
                    "FLOW::priority FLOW_PRIORITY",
                    "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                    "FLOW::priority (ANY_CHARS) (FLOW_PRIORITY)?",
                ),
                snippet=(
                    "This command is used to get/set the flow's internal packet priority.\n"
                    "Valid priority is any integer value from 0 to 7.\n"
                    "Syntax:\n"
                    "FLOW::priority [TCL handle|clientside|serverside] [priority]\n"
                    "\n"
                    "Following are the variations of this command:\n"
                    "\n"
                    "FLOW::priority\n"
                    "\n"
                    " Returns the internal packet priority of current flow.\n"
                    "\n"
                    "Flow::priority <priority>\n"
                    "\n"
                    " Sets the priority of the current flow's internal packet priority.\n"
                    " Exception is thrown if priority is outside the allowed range [0-7].\n"
                    "\n"
                    "FLOW::priority clientside\n"
                    "\n"
                    " Returns the priority of the clientside flow's internal packet priority."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "  FLOW::priority serverside 4\n"
                    "\n"
                    "  # Alternate way to use the command using the TCL flow handle.\n"
                    "  # Set priority on both client side and server side flow.\n"
                    "  FLOW::priority $clientflow 2\n"
                    "  FLOW::priority [FLOW::this] 2\n"
                    "}"
                ),
                return_value="Get operation returns an integer between 0-7. Set operation returns nothing. Exception is thrown if the operation cannot be completed.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::priority FLOW_PRIORITY",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "FLOW::priority clientside",
                                "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                            ),
                            _av(
                                "serverside",
                                "FLOW::priority serverside",
                                "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FLOW"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
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
