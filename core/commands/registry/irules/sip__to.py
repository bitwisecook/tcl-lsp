# Enriched from F5 iRules reference documentation.
"""SIP::to -- Returns the value of the To header in a SIP request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__to.html"


@register
class SipToCommand(CommandDef):
    name = "SIP::to"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::to",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of the To header in a SIP request.",
                synopsis=("SIP::to",),
                snippet="Returns the value of the To header in a SIP request.",
                source=_SOURCE,
                examples=('when SIP_REQUEST {\n    log local0.info "[SIP::to]"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::to",
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
