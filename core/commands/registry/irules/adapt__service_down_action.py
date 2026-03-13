# Enriched from F5 iRules reference documentation.
"""ADAPT::service_down_action -- Sets or returns the service-down-action attribute."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__service_down_action.html"


_av = make_av(_SOURCE)


@register
class AdaptServiceDownActionCommand(CommandDef):
    name = "ADAPT::service_down_action"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::service_down_action",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns the service-down-action attribute.",
                synopsis=(
                    "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
                ),
                snippet=(
                    "The ADAPT::service_down_action command sets or returns the\n"
                    "service-down-action attribute of the ADAPT filter on the\n"
                    "current or specified side of the virtual server connection\n"
                    "for which the iRule is being executed.\n"
                    "\n"
                    "Possible service-down-actions aare:\n"
                    "    * ignore - Do not send the HTTP request or response to the\n"
                    "      internal virtual server (bypass). Pass it through unchanged.\n"
                    "    * reset - Reset (RST) the connection.\n"
                    "    * drop - Drop (FIN) the connection."
                ),
                source=_SOURCE,
                examples=(
                    "when ADAPT_REQUEST_HEADERS {\n"
                    "     # Cause connection to be dropped if ICAP server handling\n"
                    "     # response is down for requests with a custom HTTP header\n"
                    "     # (which might have been resulted from request adaptation).\n"
                    '     if {[HTTP::header exists "X-Drop-if-down"]} {\n'
                    "        ADAPT::service_down_action response drop\n"
                    "     }\n"
                    "}"
                ),
                return_value="Returns the current or modified service-down-action.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
                    arg_values={
                        0: (
                            _av(
                                "ignore",
                                "ADAPT::service_down_action ignore",
                                "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
                            ),
                            _av(
                                "reset",
                                "ADAPT::service_down_action reset",
                                "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
                            ),
                            _av(
                                "drop",
                                "ADAPT::service_down_action drop",
                                "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"HTTP", "REQUESTADAPT", "RESPONSEADAPT"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
