# Enriched from F5 iRules reference documentation.
"""URI::query -- Returns the query string portion of the given URI or the value of a query string parameter."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__query.html"


@register
class UriQueryCommand(CommandDef):
    name = "URI::query"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::query",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the query string portion of the given URI or the value of a query string parameter.",
                synopsis=("URI::query URI_STRING (PARAMETER_NAME)?",),
                snippet=(
                    "Returns the query string portion of the given URI or the value of a\n"
                    "query string parameter."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0. "Query string of URI [HTTP::uri] is [URI::query [HTTP::uri]]"\n'
                    "}"
                ),
                return_value="Returns the query string portion of the given URI or the value of a query string parameter.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::query URI_STRING (PARAMETER_NAME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
