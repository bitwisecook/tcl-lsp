# Enriched from F5 iRules reference documentation.
"""AUTH::password_credential -- Sets the password credential to the specified string for a future AUTH::authenticate call."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__password_credential.html"


@register
class AuthPasswordCredentialCommand(CommandDef):
    name = "AUTH::password_credential"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::password_credential",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the password credential to the specified string for a future AUTH::authenticate call.",
                synopsis=("AUTH::password_credential AUTH_ID PASSWORD_CREDENTIAL",),
                snippet=(
                    "Sets the password credential to the specified string for a future\n"
                    "AUTH::authenticate call. This command returns an error if\n"
                    "attempted for a standby system.\n"
                    "\n"
                    "AUTH::password_credential authid <string>\n"
                    "\n"
                    "     * Sets the password credential to the specified string for a future\n"
                    "       AUTH::authenticate call."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n  AUTH::password_credential $auth_id [HTTP::password]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::password_credential AUTH_ID PASSWORD_CREDENTIAL",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
