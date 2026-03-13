# Enriched from F5 iRules reference documentation.
"""SSL::sessionsecret -- Return data about SSL handshake master secret."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__sessionsecret.html"


@register
class SslSessionsecretCommand(CommandDef):
    name = "SSL::sessionsecret"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::sessionsecret",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return data about SSL handshake master secret.",
                synopsis=("SSL::sessionsecret",),
                snippet="Return data about SSL handshake master secret.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0. "ClientSSL: Secret for [SSL::sessionid] is -> [SSL::sessionsecret]"\n'
                    "}"
                ),
                return_value="SSL::sessionsecret Returns the current SSL handshake master secret.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::sessionsecret",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
