# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::client_class -- Returns the classification of the client based on the current request and its browsing history."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__client_class.html"


@register
class BotdefenseClientClassCommand(CommandDef):
    name = "BOTDEFENSE::client_class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::client_class",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the classification of the client based on the current request and its browsing history.",
                synopsis=("BOTDEFENSE::client_class",),
                snippet='Returns the classification of the client that sent the request. The returned value is one of the following strings:* unknown* browser* mobile_application* trusted_bot* untrusted_bot* malicious_bot* suspicious_browser. The command is similar to BOTDEFENSE::client_type but with higher resolution for bot classification: when BOTDEFENSE::client_type returns "bot", BOTDEFENSE::client_class returns the exact type of bot: malicious, trusted or untrusted.',
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    '    log.local0. "Client type after processing request: [BOTDEFENSE::client_class]"\n'
                    "}"
                ),
                return_value='Returns the classification of the client that sent the request. When invoked in the BOTDEFENSE_REQUEST event it returns the type based on the previous requests of the same client, or "unknown" if the client is not recognized.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::client_class",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
