# Enriched from F5 iRules reference documentation.
"""DIAMETER::respond -- Sends message to client or server (based on context)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__respond.html"


@register
class DiameterRespondCommand(CommandDef):
    name = "DIAMETER::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends message to client or server (based on context).",
                synopsis=(
                    "DIAMETER::respond DIAMETER_VERSION RFLAG_BINARY PFLAG_BINARY EFLAG_BINARY TFLAG_BINARY",
                ),
                snippet=(
                    "This iRule command creates and sends a new message to the client or\n"
                    "server.\n"
                    "\n"
                    "When called from clientside events, the new message is sent to the client.\n"
                    "When called from serverside events, the new message is sent to the server."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    # DIAMETER::avp create  "avpname|code" "v" "m" "p" "vendorid" "data" "type"\n'
                    "    # 2 = DO_NOT_WANT_TO_TALK_TO_YOU\n"
                    '    set goaway [DIAMETER::avp create "disconnect-cause" 0 1 0 0 2 integer32]\n'
                    "    set version 1\n"
                    "    # 282 = Disconnect-Peer-Request\n"
                    "    set code 282\n"
                    '    set origin_host [DIAMETER::avp create "origin-host" 0 1 0 0 "bigip6.core.example.com" string]\n'
                    '    set origin_realm [DIAMETER::avp create "origin-realm" 0 1 0 0 "example.com" string]\n'
                    "    set appid 16777215"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::respond DIAMETER_VERSION RFLAG_BINARY PFLAG_BINARY EFLAG_BINARY TFLAG_BINARY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
