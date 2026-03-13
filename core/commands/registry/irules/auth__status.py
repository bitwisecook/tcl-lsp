# Enriched from F5 iRules reference documentation.
"""AUTH::status -- Returns authentication status."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__status.html"


@register
class AuthStatusCommand(CommandDef):
    name = "AUTH::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns authentication status.",
                synopsis=("AUTH::status (AUTH_ID)?",),
                snippet=(
                    "Returns authentication status. The returned status is a value of 0, 1,\n"
                    "-1, or 2, corresponding to success, failure, error, or not-authed,\n"
                    "based on the result of the most recent authorization that the system\n"
                    "performed for the specified authorization session .\n"
                    "In the case of a not-authed result, the authentication process desires\n"
                    "a credential not yet provided. Specifics of the requested credential\n"
                    "can be determined using the AUTH::wantcredential_ commands. The\n"
                    "authentication process could be continued using\n"
                    "AUTH::authenticate_continue*."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n  set authStatus [AUTH::status $authSessionId]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::status (AUTH_ID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
