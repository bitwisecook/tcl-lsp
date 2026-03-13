# Enriched from F5 iRules reference documentation.
"""ACCESS::uuid -- Enumerates the session IDs that belongs to a specified uuid key by the order of its creation and provides them in a Tcl list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__uuid.html"


@register
class AccessUuidCommand(CommandDef):
    name = "ACCESS::uuid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::uuid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enumerates the session IDs that belongs to a specified uuid key by the order of its creation and provides them in a Tcl list.",
                synopsis=(
                    "ACCESS::uuid getsid SESSION_ID",
                    "ACCESS::uuid ACCESS_UUID_COMMAND (ACCESS_UUID_INFO)?",
                ),
                snippet=(
                    "Enumerates the session IDs that belongs to a specified uuid key by the\n"
                    "order of its creation and provides them in a Tcl list. By default, the\n"
                    "uuid created by AAC is using the following format.\n"
                    "  * {profile_name}.{user_name}\n"
                    "\n"
                    "However, the admin can manually override this by specifying their own\n"
                    "uuid key via assigning that value to session.user.uuid session\n"
                    "variable. This can be done via iRule using ACCESS::session data set\n"
                    "session.user.uuid or via VPE using Variable Assignment Agent. The\n"
                    "return value of ACCESS::uuid getsid is a Tcl list."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    set apm_cookie_list [ ACCESS::uuid getsid "[PROFILE::access name].[HTTP::username]" ]\n'
                    '    log local0. "[PROFILE::access name].[HTTP::username] => session number [llength $apm_cookie_list]"\n'
                    "    for {set i 0} {$i < [llength $apm_cookie_list]} {incr i} {\n"
                    '        log local0. "MRHSession => [ lindex $apm_cookie_list $i]"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::uuid getsid SESSION_ID",
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
