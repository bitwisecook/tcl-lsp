# Enriched from F5 iRules reference documentation.
"""HTTP2::push -- Accepts a resource as a parameter that can be pushed to the client using PUSH_PROMISE frames in HTTP/2 stream."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__push.html"


@register
class Http2PushCommand(CommandDef):
    name = "HTTP2::push"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::push",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Accepts a resource as a parameter that can be pushed to the client using PUSH_PROMISE frames in HTTP/2 stream.",
                synopsis=("HTTP2::push PUSH_URI_STRING",),
                snippet=(
                    "This command has two variants.\n"
                    "\n"
                    "The first takes a requested resource, and then sends a PUSH_PROMISE frame describing that resource to the client.  The resource is requested from the server, and the payload is sent to the client on the pushed stream.\n"
                    "\n"
                    "The second method of using this command describes both the request and the response.  The request is sent as a PUSH_PROMISE to the client, and the response follows.  The server is not contacted, and the content is pushed directly from the BigIP.\n"
                    "\n"
                    "Note that this command may cause iRule events to trigger on the newly pushed stream."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    HTTP2::push /index.html host example.com\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::push PUSH_URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
