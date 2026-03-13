# Enriched from F5 iRules reference documentation.
"""ASM::username -- request username from a login attempt throughout the login session."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__username.html"


@register
class AsmUsernameCommand(CommandDef):
    name = "ASM::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="request username from a login attempt throughout the login session.",
                synopsis=("ASM::username",),
                snippet="Returns the username from a login attempt throughout the login session. If there is no login session or the login page in the policy does not extract credentials, then an empty string is returned.;",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    "                if {[ASM::is_authenticated]} {\n"
                    '                    log local0. "This request was sent by user [ASM::username]."\n'
                    "                }\n"
                    "            }"
                ),
                return_value="returns the username of an active login session or a login request.;",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::username",
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
