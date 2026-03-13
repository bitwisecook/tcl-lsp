# Enriched from F5 iRules reference documentation.
"""ACCESS::user -- Returns user ID information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__user.html"


@register
class AccessUserCommand(CommandDef):
    name = "ACCESS::user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::user",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns user ID information.",
                synopsis=(
                    "ACCESS::user getkey SID_HASH",
                    "ACCESS::user getsid KEY",
                    "ACCESS::user ACCESS_USER_COMMAND (ACCESS_USER_INFO)?",
                ),
                snippet=(
                    "The ACCESS::user commands return user ID information.\n"
                    "\n"
                    "ACCESS::user getsid <key>\n"
                    "\n"
                    "     * Returns the list of created external SIDs which is associated wit\n"
                    "       the specified key\n"
                    "\n"
                    "ACCESS::user getkey <sid_hash>\n"
                    "\n"
                    "     * Returns the original SID for specified hash of SID\n"
                    "     * This command works for clientless mode only\n"
                    "\n"
                    " * Requires APM module"
                ),
                source=_SOURCE,
                examples=(
                    "when ACCESS_SESSION_STARTED {\n"
                    "    # Associate the user_key with the session by assigning the value.\n"
                    "    if { [ info exists user_key ] } {\n"
                    '        ACCESS::session data set "session.user.uuid" $user_key\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::user getkey SID_HASH",
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
