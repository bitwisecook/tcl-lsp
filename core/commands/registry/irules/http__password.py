# Enriched from F5 iRules reference documentation.
"""HTTP::password -- Returns the password part of HTTP basic authentication."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__password.html"


@register
class HttpPasswordCommand(CommandDef):
    name = "HTTP::password"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::password",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the password part of HTTP basic authentication.",
                synopsis=("HTTP::password",),
                snippet=(
                    "Returns the password part of HTTP basic authentication.\n"
                    "As described in RFC2617 the username and password in basic\n"
                    "authentication is sent by the client in the Authorization header. The\n"
                    "client base64 encodes the username and password in the format of:\n"
                    "Authorization: Basic base64encoding(username:password)\n"
                    "The HTTP::username command parses and base64 decodes the username.\n"
                    "The HTTP::password command parses and base64 decodes the password."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n  set auth_sid [AUTH::start pam default_radius]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::password",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
