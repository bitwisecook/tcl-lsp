# Enriched from F5 iRules reference documentation.
"""SIP::from -- Returns the value of the From header in a SIP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__from.html"


@register
class SipFromCommand(CommandDef):
    name = "SIP::from"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::from",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of the From header in a SIP request.",
                synopsis=("SIP::from",),
                snippet="Returns the value of the From header in a SIP request.",
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '    log local0. "SIP Protocol - REQUEST: Values From & To"\n'
                    '    log local0. "From: [SIP::from] To: [SIP::to]"\n'
                    "}"
                ),
                return_value="Returns the value of the From header in a SIP request",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::from",
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
