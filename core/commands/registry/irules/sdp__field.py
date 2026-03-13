# Enriched from F5 iRules reference documentation.
"""SDP::field -- Gets or Sets the value in a given SDP field."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SDP__field.html"


@register
class SdpFieldCommand(CommandDef):
    name = "SDP::field"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SDP::field",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or Sets the value in a given SDP field.",
                synopsis=("SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?",),
                snippet="This command will get or set the value of a specific SDP field",
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '    log local0. "SDP field b: [SDP::field b]"\n'
                    '    SDP::field c 0 "IN IP4 10.10.1.150"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
