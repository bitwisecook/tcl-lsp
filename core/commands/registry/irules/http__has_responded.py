# Enriched from F5 iRules reference documentation.
"""HTTP::has_responded -- Returns true if this HTTP transaction has been prematurely completed by an iRule command or other filter logic."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__has_responded.html"


@register
class HttpHasRespondedCommand(CommandDef):
    name = "HTTP::has_responded"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::has_responded",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true if this HTTP transaction has been prematurely completed by an iRule command or other filter logic.",
                synopsis=("HTTP::has_responded",),
                snippet="This can be triggered by HTTP::respond, HTTP::redirect, HTTP::retry, and some ACCESS commands.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  # Used for cases where only one response to the client is permitted.\n"
                    "  # Another HTTP::respond might have been called in other iRULE script.\n"
                    "  if {[HTTP::has_responded]} {\n"
                    '    log local0. "Have already responded."\n'
                    "  } else {\n"
                    "    HTTP::respond 200 content {<html><body>First and Only Response</body></html>}\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::has_responded",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
