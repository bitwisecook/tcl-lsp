# Enriched from F5 iRules reference documentation.
"""AUTH::subscribe -- Registers interest in auth query results."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__subscribe.html"


@register
class AuthSubscribeCommand(CommandDef):
    name = "AUTH::subscribe"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::subscribe",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Registers interest in auth query results.",
                synopsis=("AUTH::subscribe AUTH_ID",),
                snippet=(
                    "AUTH::subscribe registers interest in auth query results.\n"
                    "AUTH::response_data will only return data from query results for\n"
                    "which a subscription has been made prior to calling\n"
                    "AUTH::authenticate. As a convenience when using the built-in\n"
                    "system auth rules, these rules will call AUTH::subscribe if the\n"
                    "variable tmm_auth_subscription is set. Instead of calling\n"
                    "AUTH::subscribe directly, we recommend setting tmm_auth_subscription to\n"
                    '"*" when using the built-in system auth rules in the interest of\n'
                    "forward-compatibility. Also see AUTH::unsubscribe."
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
                    synopsis="AUTH::subscribe AUTH_ID",
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
