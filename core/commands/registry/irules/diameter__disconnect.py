# Enriched from F5 iRules reference documentation.
"""DIAMETER::disconnect -- Sends Disconnect-Peer-Request to client or server based on context."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__disconnect.html"


@register
class DiameterDisconnectCommand(CommandDef):
    name = "DIAMETER::disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::disconnect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends Disconnect-Peer-Request to client or server based on context.",
                synopsis=(
                    "DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE",
                ),
                snippet=(
                    "This iRule command sends Disconnect-Peer-Request to the client (if run\n"
                    "on clientside) or to the server (if run on serverside)."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    # 2 = DO_NOT_WANT_TO_TALK_TO_YOU (RFC 6733 sec 5.4.3)\n"
                    '    DIAMETER::disconnect "bigip.core.example.com" "example.com" 2\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
