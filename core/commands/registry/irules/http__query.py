# Enriched from F5 iRules reference documentation.
"""HTTP::query -- Returns or sets the query part of the HTTP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__query.html"


@register
class HttpQueryCommand(CommandDef):
    name = "HTTP::query"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::query",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the query part of the HTTP request.",
                synopsis=("HTTP::query (QUERY_STRING)?",),
                snippet=(
                    "Returns or sets the query part of the HTTP request. The query is defined as the\n"
                    "part of the request past a ? character, if any.\n"
                    "For the following URL:\n"
                    "http://www.example.com:8080/main/index.jsp?user=test&login=check\n"
                    "The query is:\n"
                    "user=test&login=check"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "http_path [HTTP::path]"\n'
                    '  log local0. "http_query [HTTP::query]"\n'
                    "  HTTP::query user=test_user&login=test_login\n"
                    "}"
                ),
                return_value="Returns the query part of the HTTP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::query",
                    arity=Arity(0, 0),
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.HTTP_URI,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                            scope=StorageScope.EVENT,
                        ),
                    ),
                ),
                FormSpec(
                    kind=FormKind.SETTER,
                    synopsis="HTTP::query <QUERY_STRING>",
                    arity=Arity(1, 1),
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.HTTP_URI,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                            scope=StorageScope.EVENT,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
