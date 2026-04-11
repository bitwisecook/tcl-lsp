# Enriched from F5 iRules reference documentation.
"""HTTP2::requests -- This command can be used to determine the count of requests received in the current HTTP/2 session."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__requests.html"


@register
class Http2RequestsCommand(CommandDef):
    name = "HTTP2::requests"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::requests",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to determine the count of requests received in the current HTTP/2 session.",
                synopsis=("HTTP2::requests",),
                snippet="Returns the count of requests received in the current HTTP/2 session. This includes the current request. Returns 0 if HTTP/2 is not active.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0. "Number of requests received in current session is [HTTP2::requests]"\n'
                    "}"
                ),
                return_value="The return is a number indicating count of requests received in current HTTP/2 session. The return is 0 if HTTP/2 is not active.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::requests",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
