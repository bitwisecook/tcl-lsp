# Enriched from F5 iRules reference documentation.
"""HTTP2::disconnect -- This command allows you to cleanly terminate the current HTTP/2 session."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__disconnect.html"


@register
class Http2DisconnectCommand(CommandDef):
    name = "HTTP2::disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::disconnect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to cleanly terminate the current HTTP/2 session.",
                synopsis=("HTTP2::disconnect",),
                snippet="Cleanly terminates the current HTTP/2 session, if HTTP/2 is active.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if {[HTTP2::active]} {\n"
                    "        HTTP2::disconnect\n"
                    "    }\n"
                    "}"
                ),
                return_value="The return is 0 if the disconnect was clean. If a GOAWAY frame cannot be sent, an error will be returned.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::disconnect",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
