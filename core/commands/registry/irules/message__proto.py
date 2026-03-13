# Enriched from F5 iRules reference documentation.
"""MESSAGE::proto -- Returns protocol of the message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MESSAGE__proto.html"


@register
class MessageProtoCommand(CommandDef):
    name = "MESSAGE::proto"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MESSAGE::proto",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns protocol of the message.",
                synopsis=("MESSAGE::proto",),
                snippet=(
                    "returns protocol of the message. For example, SIP, and DIAMETER.\n"
                    "This is valid for messages of the following protocols:\n"
                    "\n"
                    "    DIAMETER\n"
                    "    SIP"
                ),
                source=_SOURCE,
                examples=('when MR_INGRESS {\n    log local0. "[MESSAGE::proto]"\n}'),
                return_value="returns protocol of the message. For example, SIP, and DIAMETER.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MESSAGE::proto",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
