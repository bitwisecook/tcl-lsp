# Enriched from F5 iRules reference documentation.
"""AUTH::last_event_session_id -- Returns the session ID of the last auth event."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__last_event_session_id.html"


@register
class AuthLastEventSessionIdCommand(CommandDef):
    name = "AUTH::last_event_session_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::last_event_session_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the session ID of the last auth event.",
                synopsis=("AUTH::last_event_session_id",),
                snippet=(
                    "This command returns the session ID of the last auth event, which can\n"
                    "then be used to relate to the user behind each session.\n"
                    "\n"
                    "AUTH::last_event_session_id\n"
                    "\n"
                    "     * Returns the session ID of the last auth event"
                ),
                source=_SOURCE,
                examples=(
                    "when AUTH_SUCCESS {\n"
                    "  if {$auth_id eq [AUTH::last_event_session_id]} {\n"
                    '    log local0. "auth success event"\n'
                    "    set authorized 1\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::last_event_session_id",
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
