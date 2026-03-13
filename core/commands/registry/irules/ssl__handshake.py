# Enriched from F5 iRules reference documentation.
"""SSL::handshake -- Halts or resumes SSL activity."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__handshake.html"


_av = make_av(_SOURCE)


@register
class SslHandshakeCommand(CommandDef):
    name = "SSL::handshake"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::handshake",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Halts or resumes SSL activity.",
                synopsis=("SSL::handshake (hold | resume)",),
                snippet="Halts or resumes SSL activity. This is useful for suspending SSL activity while authentication is in progress.",
                source=_SOURCE,
                examples=(
                    "when AUTH_ERROR {\n"
                    "    if {$auth_ldap_sid eq [AUTH::last_event_session_id]} {\n"
                    "        reject\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::handshake hold Halts any SSL activity. Typically used when an authentication request is made.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::handshake (hold | resume)",
                    arg_values={
                        0: (
                            _av("hold", "SSL::handshake hold", "SSL::handshake (hold | resume)"),
                            _av(
                                "resume", "SSL::handshake resume", "SSL::handshake (hold | resume)"
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
                    target=SideEffectTarget.SSL_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
