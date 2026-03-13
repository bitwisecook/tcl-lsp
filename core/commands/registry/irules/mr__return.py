# Enriched from F5 iRules reference documentation.
"""MR::return -- Returns the current message to the originating connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__return.html"


_av = make_av(_SOURCE)


@register
class MrReturnCommand(CommandDef):
    name = "MR::return"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::return",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current message to the originating connection.",
                synopsis=(
                    "MR::return",
                    "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                ),
                snippet=(
                    "The MR::return command instructs the Message Routing Framework to return the current message to the originating connection. The message's route status will be updated to 'returned by irule' or the provided route status. When the connection is received on the originating connection, MR_FAILED event will be raised.\n"
                    "        \n"
                    "Returns the current message to the originating connection with a route status of 'returned by irule'\n"
                    "            \n"
                    "Returns the current message to the originating connection and sets the route status to the route status specified."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    if {[DIAMETER::is_response]} {\n"
                    "        incr pend_req -1\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::return",
                    arg_values={
                        0: (
                            _av(
                                "no_route_found",
                                "MR::return no_route_found",
                                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                            ),
                            _av(
                                "queue_full",
                                "MR::return queue_full",
                                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                            ),
                            _av(
                                "no_connection",
                                "MR::return no_connection",
                                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                            ),
                            _av(
                                "connection_closing",
                                "MR::return connection_closing",
                                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                            ),
                            _av(
                                "internal_error",
                                "MR::return internal_error",
                                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                            ),
                            _av(
                                "max_retries_exceeded",
                                "MR::return max_retries_exceeded",
                                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
