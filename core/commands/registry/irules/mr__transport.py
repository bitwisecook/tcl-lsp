# Enriched from F5 iRules reference documentation.
"""MR::transport -- Returns the name and type (virtual or config) of the transport used to configure the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__transport.html"


@register
class MrTransportCommand(CommandDef):
    name = "MR::transport"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::transport",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name and type (virtual or config) of the transport used to configure the current connection.",
                synopsis=("MR::transport",),
                snippet="Returns the name and type (virtual or config) of the transport used to configure the current connection. These values can be used to generate routes or to set the route of a message.",
                source=_SOURCE,
                examples=(
                    'when MR_EGRESS {\n    log local0. "sending message via [MR::transport]"\n}'
                ),
                return_value="Returns the name and type (virtual or config) of the transport used to configure the current connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::transport",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
