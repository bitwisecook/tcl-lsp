# Enriched from F5 iRules reference documentation.
"""HTTP::path -- Returns or sets the path part of the HTTP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import SetterConstraint, TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__path.html"


@register
class HttpPathCommand(CommandDef):
    name = "HTTP::path"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::path",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the path part of the HTTP request.",
                synopsis=("HTTP::path (PATH_VALUE)?",),
                snippet=(
                    "Returns or sets the path part of the HTTP request. The path is defined\n"
                    "as the path and filename in a request. It does not include the query string.\n"
                    "For the following URL: http://www.example.com:8080/main/index.jsp?user=test&login=check\n"
                    "\n"
                    "The path is: /main/index.jsp\n"
                    "\n"
                    "Note that only ? is used as the separator for the query string.\n"
                    "So, for the following URL: http://www.example.com:8080/main/index.jsp;session_id=abcdefgh?user=test&login=check\n"
                    "\n"
                    "The path is: /main/index.jsp;session_id=abcdefgh\n"
                    "\n"
                    "**Best practice**: Prefer `HTTP::path` over `HTTP::uri` when you\n"
                    "only need the path without the query string. Use `-normalized`\n"
                    "for consistent matching (prevents URL-encoding evasion)."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "\\[HTTP::path\\] original: [HTTP::path]"\n'
                    "  HTTP::path [string tolower [HTTP::path]]\n"
                    "}"
                ),
                return_value="Returns the path part of the HTTP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::path",
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
                    synopsis="HTTP::path <PATH_VALUE>",
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
            diagram_action=True,
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={
                # Getter form (0 args): returns a path-prefixed tainted value.
                Arity(0, 0): TaintColour.TAINTED | TaintColour.PATH_PREFIXED,
            },
            setter_constraints=(
                SetterConstraint(
                    arg_index=0,
                    required_prefix="/",
                    code="IRULE3101",
                    message="HTTP::path value must start with '/'",
                ),
            ),
        )
