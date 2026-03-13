# Enriched from F5 iRules reference documentation.
"""AUTH::start -- Initializes an authentication session."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__start.html"


@register
class AuthStartCommand(CommandDef):
    name = "AUTH::start"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::start",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Initializes an authentication session.",
                synopsis=("AUTH::start TYPE SERVICE",),
                snippet=(
                    "Initializes an authentication session. This command returns the\n"
                    "authentication session ID, which must be specified to other\n"
                    "authentication commands. Multiple simultaneous authentication sessions\n"
                    "(up to 10) can be opened for a single connection, but it is the user’s\n"
                    "responsibility to keep track of their respective session IDs. This\n"
                    "command returns an error if attempted for a standby system.\n"
                    "\n"
                    "AUTH::start <type> <PAM service>\n"
                    "\n"
                    "     * Returns the authentication session ID, which must be specified to\n"
                    "       other authentication commands."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n  set auth_id [AUTH::start pam default_radius]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::start TYPE SERVICE",
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
