# Enriched from F5 iRules reference documentation.
"""SCTP::respond -- Sends the specified data directly to the peer."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__respond.html"


@register
class SctpRespondCommand(CommandDef):
    name = "SCTP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends the specified data directly to the peer.",
                synopsis=("SCTP::respond (PAYLOAD_DATA (PAYLOAD_OFFSET)? (PAYLOAD_LENGTH)?)",),
                snippet="Sends the specified data directly to the peer. This command is used to complete a protocol handshake with an iRule.",
                source=_SOURCE,
                examples=('when CLIENT_ACCEPTED {\n    SCTP::respond "sctpdata" 0 1\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::respond (PAYLOAD_DATA (PAYLOAD_OFFSET)? (PAYLOAD_LENGTH)?)",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
