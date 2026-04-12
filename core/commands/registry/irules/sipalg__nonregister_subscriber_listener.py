# Enriched from F5 iRules reference documentation.
"""SIPALG::nonregister_subscriber_listener -- Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIPALG__nonregister_subscriber_listener.html"


@register
class SipalgNonregisterSubscriberListenerCommand(CommandDef):
    name = "SIPALG::nonregister_subscriber_listener"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIPALG::nonregister_subscriber_listener",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers.",
                synopsis=(
                    "SIPALG::nonregister_subscriber_listener",
                    "SIPALG::nonregister_subscriber_listener (BOOLEAN)",
                ),
                snippet="Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers.",
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '    log local0. "nonregister_subscriber_listener is [SIPALG::nonregister_subscriber_listener]"\n'
                    "}"
                ),
                return_value="Returns 1, or 0",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIPALG::nonregister_subscriber_listener",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"SIP"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
