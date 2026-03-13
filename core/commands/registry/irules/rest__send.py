# Generated from F5 iRules reference documentation -- do not edit manually.
"""REST::send -- Send a rest request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REST__send.html"


@register
class RestSendCommand(CommandDef):
    name = "REST::send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REST::send",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Send a rest request.",
                synopsis=("REST::send",),
                snippet="Send a rest request locally to the Big-IP REST Framework",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REST::send",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
