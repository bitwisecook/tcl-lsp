# Enriched from F5 iRules reference documentation.
"""SIP::call_id -- Returns the value of the Call-ID header in a SIP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__call_id.html"


@register
class SipCallIdCommand(CommandDef):
    name = "SIP::call_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::call_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of the Call-ID header in a SIP request.",
                synopsis=("SIP::call_id",),
                snippet=(
                    "Returns the value of the Call-ID header in a SIP request. Only the\n"
                    "first 256 bytes of the Call-ID will be returned."
                ),
                source=_SOURCE,
                examples=('when SIP_REQUEST_SEND {\n    log local0. "Call ID [SIP::call_id]"\n}'),
                return_value="Returns the value of the Call-ID header in a SIP request",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::call_id",
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
