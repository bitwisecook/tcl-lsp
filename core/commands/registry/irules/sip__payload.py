# Enriched from F5 iRules reference documentation.
"""SIP::payload -- Returns the accumulated SIP data content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__payload.html"


@register
class SipPayloadCommand(CommandDef):
    name = "SIP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the accumulated SIP data content.",
                synopsis=(
                    "SIP::payload (LENGTH | (OFFSET LENGTH))?",
                    "SIP::payload length",
                    "SIP::payload replace OFFSET LENGTH PAYLOAD",
                    "SIP::payload insert OFFSET PAYLOAD",
                ),
                snippet="Returns the accumulated SIP data content.",
                source=_SOURCE,
                examples=(
                    'when SIP_REQUEST {\n    log local0. "unmodified request [SIP::payload]"\n}'
                ),
                return_value="Returns the SIP data accumulated so far",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::payload (LENGTH | (OFFSET LENGTH))?",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
