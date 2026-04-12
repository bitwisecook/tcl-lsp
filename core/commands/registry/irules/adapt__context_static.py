# Enriched from F5 iRules reference documentation.
"""ADAPT::context_static -- Gets the static context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ADAPT__context_static.html"


@register
class AdaptContextStaticCommand(CommandDef):
    name = "ADAPT::context_static"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ADAPT::context_static",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the static context.",
                synopsis=("ADAPT::context_static (ADAPT_SIDE)?",),
                snippet=(
                    "Obtains a handle for the static context on the current\n"
                    "or specified side. The static context is the profile-based\n"
                    "context that applies when there are no dynamic contexts on that\n"
                    "side. Returns a null string if the connection flow has not\n"
                    "yet been initialized (for example, if the command was issued\n"
                    "from a request-adapt (client side) event and the server side\n"
                    "connection has not yet been established).\n"
                    "\n"
                    "Syntax:\n"
                    "\n"
                    "ADAPT::context_static\n"
                    "\n"
                    "    * Gets the static context on the current side.\n"
                    "\n"
                    "ADAPT::context_static request\n"
                    "\n"
                    "    * Gets the static context on the request-adapt side."
                ),
                source=_SOURCE,
                examples=(
                    "when ADAPT_REQUEST_RESULT {\n"
                    "    set static_ctx [ADAPT::context_static]\n"
                    "    set ctx [ADAPT::context_current]\n"
                    "    if {$ctx == $static_ctx} {\n"
                    '        log local0. "No dynamic contexts have been created."\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns the handle of the static context, or a null string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ADAPT::context_static (ADAPT_SIDE)?",
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
