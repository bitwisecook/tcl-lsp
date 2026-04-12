# Enriched from F5 iRules reference documentation.
"""AUTH::authenticate -- Performs a new authentication operation."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__authenticate.html"


@register
class AuthAuthenticateCommand(CommandDef):
    name = "AUTH::authenticate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::authenticate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Performs a new authentication operation.",
                synopsis=("AUTH::authenticate AUTH_ID",),
                snippet=(
                    "Performs a new authentication operation. This command returns an error\n"
                    "if attempted for a standby system or while an authentication operation\n"
                    "is already in progress for this authentication session.\n"
                    "\n"
                    "AUTH::authenticate <authid>\n"
                    "\n"
                    "     * Performs a new authentication operation. This command returns an\n"
                    "       error if attempted for a standby system or while an authentication\n"
                    "       operation is already in progress for this authentication session."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  AUTH::username_credential $auth_id [HTTP::username]\n"
                    "  AUTH::password_credential $auth_id [HTTP::password]\n"
                    "  AUTH::authenticate $auth_id\n"
                    "  HTTP::collect\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::authenticate AUTH_ID",
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
