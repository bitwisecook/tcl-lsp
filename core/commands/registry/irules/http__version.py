# Enriched from F5 iRules reference documentation.
"""HTTP::version -- Returns or sets the HTTP version of the request or response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__version.html"


@register
class HttpVersionCommand(CommandDef):
    name = "HTTP::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::version",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or sets the HTTP version of the request or response.",
                synopsis=(
                    "HTTP::version ('0.9' | '1.0' | '1.1')?",
                    "HTTP::version '-string' (ANY_CHARS)?",
                ),
                snippet=(
                    "Returns or sets the HTTP version of the request or response. This\n"
                    "command replaces the BIG-IP 4.X variable http_version.\n"
                    "If needed, Connection and Host headers will automatically be added\n"
                    "appropriately.\n"
                    "HTTP::version will return the original version of the request or\n"
                    "response, even if it has been changed.  Note that this will return\n"
                    'the "effective" version used, which may be different than the actual\n'
                    "version string in the request or response.  For example, invalid\n"
                    "version numbers may be parsed as 1.1 in order to increase\n"
                    "inter-operability with common HTTP servers."
                ),
                source=_SOURCE,
                examples=('when HTTP_RESPONSE {\n  HTTP::version "1.1"\n}'),
                return_value="Returns the HTTP version of the request or response",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::version ('0.9' | '1.0' | '1.1')?",
                    options=(
                        OptionSpec(name="-string", detail="Option -string.", takes_value=True),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
