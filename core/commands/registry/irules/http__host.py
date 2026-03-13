# Enriched from F5 iRules reference documentation.
"""HTTP::host -- F5 iRules command `HTTP::host`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__host.html"


@register
class HttpHostCommand(CommandDef):
    name = "HTTP::host"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::host",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the value contained in the Host header of an HTTP request.",
                synopsis=("HTTP::host",),
                snippet=(
                    "Returns the value contained in the Host header of an HTTP request. This\n"
                    "command replaces the BIG-IP 4.X variable http_host.\n"
                    "The Host header always contains the requested host name (which may be a\n"
                    "Host Domain Name string or an IP address), and will also contain the\n"
                    "requested service port whenever a non-standard port is specified (other\n"
                    "than 80 for HTTP, other than 443 for HTTPS). When present, the\n"
                    "non-standard port is appended to the requsted name as a numeric string\n"
                    "with a colon separating the 2 values (just as it would appear in the\n"
                    "browser's address bar):\n"
                    "  * Host: host.domain."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  if { [HTTP::uri] contains "secure"} {\n'
                    '    HTTP::redirect "https://[HTTP::host][HTTP::uri]"\n'
                    " }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::host",
                    arity=Arity(0, 0),
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.HTTP_HEADER,
                            reads=True,
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
