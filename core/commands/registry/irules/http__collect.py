# Enriched from F5 iRules reference documentation.
"""HTTP::collect -- Collects an amount of HTTP body data that you specify."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__collect.html"


@register
class HttpCollectCommand(CommandDef):
    name = "HTTP::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collects an amount of HTTP body data that you specify.",
                synopsis=("HTTP::collect (CONTENT_LENGTH)?",),
                snippet=(
                    "Collects an amount of HTTP body data, optionally specified with\n"
                    "the <length> argument. When the system collects the specified\n"
                    "amount of data, it calls the Tcl event HTTP_REQUEST_DATA or\n"
                    "HTTP_RESPONSE_DATA. The collected data can be accessed via the\n"
                    "HTTP::payload command.\n"
                    "\n"
                    "Note that this command cannot be called after any Tcl command that\n"
                    "sends an HTTP response (e.g. redirect, HTTP::redirect, and\n"
                    "HTTP::respond). A run-time error will result.\n"
                    "\n"
                    "Care must be taken when using HTTP::collect to not stall the\n"
                    "connection.\n"
                    "\n"
                    "**Caution**: `HTTP::collect` cannot be called twice on the same\n"
                    "connection. A second call will fail or cause a TCL error. Use a\n"
                    "state variable (e.g. `set http_state collect`) to guard against\n"
                    "double-collect across multiple iRules."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST_DATA {\n"
                    "  # do stuff with the payload\n"
                    "  set payload [HTTP::payload]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::collect (CONTENT_LENGTH)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_BODY,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
