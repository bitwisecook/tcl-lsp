# Enriched from F5 iRules reference documentation.
"""HTTP2::concurrency -- This command can be used to determine the number of active concurrent streams in the current HTTP/2 session."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP2__concurrency.html"


@register
class Http2ConcurrencyCommand(CommandDef):
    name = "HTTP2::concurrency"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP2::concurrency",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to determine the number of active concurrent streams in the current HTTP/2 session.",
                synopsis=("HTTP2::concurrency",),
                snippet="Returns number of active concurrent streams in the current HTTP/2 session.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0. "Number of active concurrent streams is [HTTP2::concurrency]"\n'
                    "}"
                ),
                return_value="The return is a number indicating the active concurrent streams in current HTTP/2 session.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP2::concurrency",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP2"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP2_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
