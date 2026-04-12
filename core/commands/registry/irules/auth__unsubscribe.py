# Enriched from F5 iRules reference documentation.
"""AUTH::unsubscribe -- Cancels interest in auth query results."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__unsubscribe.html"


@register
class AuthUnsubscribeCommand(CommandDef):
    name = "AUTH::unsubscribe"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::unsubscribe",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Cancels interest in auth query results.",
                synopsis=("AUTH::unsubscribe AUTH_ID",),
                snippet=(
                    "AUTH::unsubscribe cancels interest in auth query results.\n"
                    "AUTH::response_data will not return data from query results for\n"
                    "which a subscription has been cancelled before AUTH::authenticate\n"
                    "has been called. Also see AUTH::subscribe.\n"
                    "\n"
                    "AUTH::unsubscribe <authid>\n"
                    "\n"
                    "     * Cancels interest in auth query results."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "        if {not [info exists auth_pass]} {\n"
                    "            set auth_sid [AUTH::start pam auth_method_user]\n"
                    "            AUTH::subscribe $auth_sid\n"
                    "            set auth_username [HTTP::username]\n"
                    "            set auth_password [HTTP::password]\n"
                    "            AUTH::username_credential $auth_sid $auth_username\n"
                    "            AUTH::password_credential $auth_sid $auth_password\n"
                    "            AUTH::authenticate $auth_sid\n"
                    "            set auth_pass 1\n"
                    "        }\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::unsubscribe AUTH_ID",
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
