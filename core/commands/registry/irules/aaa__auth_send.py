# Enriched from F5 iRules reference documentation.
"""AAA::auth_send -- This command is used to send user authentication information to IVS(internal virtual server)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AAA__auth_send.html"


@register
class AaaAuthSendCommand(CommandDef):
    name = "AAA::auth_send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AAA::auth_send",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to send user authentication information to IVS(internal virtual server).",
                synopsis=("AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?",),
                snippet="This command is used to send user authentication information to IVS(internal virtual server).",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST_DATA {\n"
                    "    set request_id [AAA::auth_send $internal_radius_aaa_vip $username $password]\n"
                    "\n"
                    "    set aaa_result [AAA::auth_result $request_id]\n"
                    '    if { $aaa_result == "OK" } {\n'
                    "        # request was successfull\n"
                    "    } else {\n"
                    "        # handle errors\n"
                    "    }\n"
                    "}"
                ),
                return_value="request_id - the id of the current connection that can be used to check the status later with AAA::auth_result command",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?",
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
