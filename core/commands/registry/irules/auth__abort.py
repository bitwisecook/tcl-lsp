# Enriched from F5 iRules reference documentation.
"""AUTH::abort -- Cancels any outstanding auth operations in this authentication session."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__abort.html"


@register
class AuthAbortCommand(CommandDef):
    name = "AUTH::abort"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::abort",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Cancels any outstanding auth operations in this authentication session.",
                synopsis=("AUTH::abort AUTH_ID",),
                snippet=(
                    "Cancels any outstanding auth operations in this authentication session,\n"
                    "and generates an AUTH_FAILURE event if there was an outstanding\n"
                    "authentication query in progress. This command invalidates the\n"
                    "specified authentication session ID, which should be discarded upon\n"
                    "calling this command.\n"
                    "\n"
                    "AUTH::abort authid\n"
                    "\n"
                    "     * Cancels any outstanding auth operations in this authentication\n"
                    "       session, and generates an AUTH_FAILURE event if there was an\n"
                    "       outstanding authentication query in progress."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set auth_http_successes 0\n"
                    "    set auth_http_sufficient_successes 2\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::abort AUTH_ID",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
