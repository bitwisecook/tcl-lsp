# Enriched from F5 iRules reference documentation.
"""SSL::sessionticket -- Returns the session ticket associated with the SSL flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__sessionticket.html"


@register
class SslSessionticketCommand(CommandDef):
    name = "SSL::sessionticket"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::sessionticket",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the session ticket associated with the SSL flow.",
                synopsis=("SSL::sessionticket",),
                snippet="This command returns the session ticket associated with the SSL flow.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    "    set st [SSL::sessionticket]\n"
                    "    set stlen [string length $st]\n"
                    '    log local0. "stlen $stlen"\n'
                    '    log local0. "st $st"\n'
                    "}"
                ),
                return_value="This command returns the session ticket associated with the SSL flow.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::sessionticket",
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
