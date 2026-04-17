# Enriched from F5 iRules reference documentation.
"""SSE::field -- Basic access and manipulation of SSE message fields"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSE__field.html"


@register
class SseFieldCommand(CommandDef):
    name = "SSE::field"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSE::field",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Basic access and manipulation of SSE message fields",
                synopsis=("SSE::field (",),
                snippet="This command provides the ability to create, read, update, and delete individual fields on a Server Sent Events (SSE) message.",
                source=_SOURCE,
                examples=(
                    "when SSE_RESPONSE {\n"
                    '    log local0. "event is " [SSE::field get event]\n'
                    "\n"
                    "    SSE::field set my-custom-field  my-custom-value\n"
                    '    log local0. "my-custom-field is " [SSE::field get my-custom-field]\n'
                    "\n"
                    "    SSE::field remove event\n"
                    "}"
                ),
                return_value="'get' returns the value of the specified field name. If field name does not exist, a null string is returned. 'set' and 'remove' do not return a value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSE::field (",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
