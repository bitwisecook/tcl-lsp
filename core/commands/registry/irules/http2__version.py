# Enriched from F5 iRules reference documentation.
"""HTTP2::version -- This command can be used to determine the HTTP/2 protocol version used."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__version.html"


@register
class Http2VersionCommand(CommandDef):
    name = "HTTP2::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::version",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to determine the HTTP/2 protocol version used.",
                synopsis=("HTTP2::version",),
                snippet="Returns 2 if the HTTP/2 protocol is used. Returns 0 if no HTTP/2 request is active.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if {[HTTP2::version] != 0} {\n"
                    '        HTTP::header insert "X-HTTP2-Values version " "[HTTP2::version]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="The return is 2 if the HTTP/2 protocol is used, 0 if HTTP/2 is not active.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::version",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP"}),
                also_in=frozenset({"MR_INGRESS"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
