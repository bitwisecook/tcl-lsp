# Enriched from F5 iRules reference documentation.
"""AUTH::username_credential -- Sets the username credential to a string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__username_credential.html"


@register
class AuthUsernameCredentialCommand(CommandDef):
    name = "AUTH::username_credential"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::username_credential",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the username credential to a string.",
                synopsis=("AUTH::username_credential AUTH_ID USERNAME_CREDENTIAL",),
                snippet=(
                    "Sets the username credential to the specified string, for a future\n"
                    "AUTH::authenticate call. This command returns an error if\n"
                    "attempted for a standby system.\n"
                    "\n"
                    "AUTH::username_credential authid <string>\n"
                    "\n"
                    "     * Sets the username credential to the specified string, for a future\n"
                    "       AUTH::authenticate call."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n  AUTH::username_credential $asid [HTTP::username]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::username_credential AUTH_ID USERNAME_CREDENTIAL",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
