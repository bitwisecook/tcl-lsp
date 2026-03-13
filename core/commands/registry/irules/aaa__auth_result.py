# Enriched from F5 iRules reference documentation.
"""AAA::auth_result -- This command is used to check the result of an authentication request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AAA__auth_result.html"


@register
class AaaAuthResultCommand(CommandDef):
    name = "AAA::auth_result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AAA::auth_result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to check the result of an authentication request.",
                synopsis=("AAA::auth_result AAA_REQUEST_ID",),
                snippet="This command is used to check the result of an authentication request. It can be used to determine whether the user was successfully authenticated, or if the authentication failed or if the system encountered an error.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST_DATA {\n"
                    "    set aaa_result [AAA::auth_result $request_id]\n"
                    '    if { $aaa_result == "INPROGRESS" } {\n'
                    "        after 200\n"
                    "        continue\n"
                    "    }\n"
                    "\n"
                    '    if { $aaa_result == "OK" } {\n'
                    "        # request was successfull\n"
                    "    } else {\n"
                    "        # handle errors\n"
                    "    }\n"
                    "}"
                ),
                return_value='There are 4 possible return values for this command (All STRING type): "OK" - User was successfully authenticated "FAIL" - Authentication failed "INPROGRESS" - the request is still in progress (asyncronous). "ERROR" - there was an error during the request.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AAA::auth_result AAA_REQUEST_ID",
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
