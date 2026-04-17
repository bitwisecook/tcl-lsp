# Enriched from F5 iRules reference documentation.
"""URI::port -- Returns the host port from the given URI."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__port.html"


@register
class UriPortCommand(CommandDef):
    name = "URI::port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the host port from the given URI.",
                synopsis=("URI::port URI_STRING",),
                snippet="Returns the host port from the given URI.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  set port [URI::port [HTTP::uri]]\n"
                    '  log local0. "Host port of uri [HTTP::uri] is $port"\n'
                    "}"
                ),
                return_value="Returns the host port from the given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::port URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
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
