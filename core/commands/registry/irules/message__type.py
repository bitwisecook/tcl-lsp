# Enriched from F5 iRules reference documentation.
"""MESSAGE::type -- Returns the type of the current message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MESSAGE__type.html"


@register
class MessageTypeCommand(CommandDef):
    name = "MESSAGE::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MESSAGE::type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the type of the current message.",
                synopsis=("MESSAGE::type",),
                snippet=(
                    "returns the type of the current message. For example, for SIP request, it will return 'request'.\n"
                    "This is valid for messages of the following protocols:\n"
                    "\n"
                    "    DIAMETER\n"
                    "    SIP"
                ),
                source=_SOURCE,
                examples=(
                    'when MR_INGRESS {\n    log local0. "[MESSAGE::proto] [MESSAGE::type]"\n}'
                ),
                return_value="returns the type of the current message. For example, for SIP request, it will return 'request'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MESSAGE::type",
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
