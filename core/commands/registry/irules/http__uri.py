# Enriched from F5 iRules reference documentation.
"""HTTP::uri -- Returns or sets the URI part of the HTTP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import SetterConstraint, TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__uri.html"


@register
class HttpUriCommand(CommandDef):
    name = "HTTP::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::uri",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the URI part of the HTTP request.",
                synopsis=("HTTP::uri (URI)?",),
                snippet=(
                    "Returns or sets the URI part of the HTTP request. This command replaces\n"
                    "the BIG-IP 4.X variable http_uri.\n"
                    "\n"
                    "For the following URL:\n"
                    "http://www.example.com:8080/main/index.jsp?user=test&login=check\n"
                    "The URI is: /main/index.jsp?user=test&login=check\n"
                    "\n"
                    "Note that in the HTTP_PROXY_REQUEST event, this command returns the complete\n"
                    "proxy URI. This includes the scheme, host and port, and thus the result would be:\n"
                    "http://www.example.com:8080/main/index.jsp?user=test&login=check"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_PROXY_REQUEST {\n"
                    '   log local.0 "This proxy request is:[HTTP::uri]"\n'
                    "}"
                ),
                return_value="Returns the URI part of the HTTP request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::uri",
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
                    synopsis="HTTP::uri <URI>",
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
                    message="HTTP::uri value must start with '/'",
                ),
            ),
        )
