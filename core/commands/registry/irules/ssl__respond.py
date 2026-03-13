# Enriched from F5 iRules reference documentation.
"""SSL::respond -- Return data back to the origin via SSL."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__respond.html"


@register
class SslRespondCommand(CommandDef):
    name = "SSL::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return data back to the origin via SSL.",
                synopsis=("SSL::respond DATA",),
                snippet="Returns the specified plaintext data back to the origin over the encrypted SSL connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    "              # Trigger collection of the decrypted payload once the SSL or DTLS handshake has been completed successfully\n"
                    "              SSL::collect\n"
                    "            }"
                ),
                return_value="SSL::respond <data> Returns the specified plaintext data back to the origin over the encrypted SSL connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::respond DATA",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
