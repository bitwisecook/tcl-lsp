# Enriched from F5 iRules reference documentation.
"""SIP::message -- Returns the full content of the SIP request or response message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__message.html"


@register
class SipMessageCommand(CommandDef):
    name = "SIP::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the full content of the SIP request or response message.",
                synopsis=("SIP::message",),
                snippet=(
                    "The SIP::message command returns full content of the SIP request\n"
                    "or response message."
                ),
                source=_SOURCE,
                examples=("when SIP_REQUEST {\n  log local0. [SIP::message]\n}"),
                return_value="Returns content of the current message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::message",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
