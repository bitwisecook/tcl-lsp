# Enriched from F5 iRules reference documentation.
"""AAA::acct_send -- This command is used to send user accouting information to IVS(internal virtual server)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AAA__acct_send.html"


_av = make_av(_SOURCE)


@register
class AaaAcctSendCommand(CommandDef):
    name = "AAA::acct_send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AAA::acct_send",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to send user accouting information to IVS(internal virtual server).",
                synopsis=("AAA::acct_send VIRTUAL_SERVER ((('user-name' USERNAME)",),
                snippet=(
                    "This command is used to send user accouting information to IVS(internal virtual server). The accounting information can be identified by one or more of the following attributes:\n"
                    "    - user-name\n"
                    "    - framed-ip-address\n"
                    "    - framed-ipv6-prefix\n"
                    "    - event-timestamp\n"
                    "    - acct-status-type\n"
                    "    - acct-session-id\n"
                    "    - acct-input-octets\n"
                    "    - acct-output-octets\n"
                    "    - 3gpp-imsi\n"
                    "    - 3gpp-imeisv\n"
                    "    - 3gpp-user-location-info\n"
                    "\n"
                    "Syntax:"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST_DATA {\n"
                    "    set request_id [AAA::acct_send $internal_radius_aaa_vip user-name $username\n"
                    "                                                            framed-ip-address $framed-ip\n"
                    "                                                            acct-status-type 1]\n"
                    "\n"
                    "    set aaa_result [AAA::acct_result $request_id]\n"
                    '    if { $aaa_result == "OK" } {\n'
                    "        # request was successfull\n"
                    "    } else {\n"
                    "        # handle errors\n"
                    "    }\n"
                    "}"
                ),
                return_value="request_id - the id of the current connection that can be used to check the status later with AAA::acct_result command",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AAA::acct_send VIRTUAL_SERVER ((('user-name' USERNAME)",
                    arg_values={
                        0: (
                            _av(
                                "user-name",
                                "AAA::acct_send user-name",
                                "AAA::acct_send VIRTUAL_SERVER ((('user-name' USERNAME)",
                            ),
                        )
                    },
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
