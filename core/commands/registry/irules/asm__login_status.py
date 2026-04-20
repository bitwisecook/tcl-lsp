# Enriched from F5 iRules reference documentation.
"""ASM::login_status -- Request status of the login session tracked by one of the login pages defined in the policy."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__login_status.html"


@register
class AsmLoginStatusCommand(CommandDef):
    name = "ASM::login_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::login_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Request status of the login session tracked by one of the login pages defined in the policy.",
                synopsis=("ASM::login_status",),
                snippet=(
                    "Returns status of the login session tracked by one of the login pages defined in the policy. Following are the possible values:\n"
                    "\n"
                    "                not_logged_in: The request is not within a login session.\n"
                    "                logging_in: The request is to a login URL.\n"
                    "                logged_in: The request is within a login session, indicates a successful login in the ASM_RESPONSE_LOGIN event.\n"
                    "                failed: The login attempt is failed, triggered only in the ASM_RESPONSE_LOGIN event."
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_RESPONSE_LOGIN {\n"
                    '                if {[ASM::login_status] eq "logged_in"} {\n'
                    '                    log local0. "User [ASM::username] logged in succesfully."\n'
                    "                }\n"
                    "                else {\n"
                    '                    log local0. "Login attempt to [ASM::username] failed."\n'
                    "                }\n"
                    "            }"
                ),
                return_value="Returns status of the login session.;",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::login_status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
