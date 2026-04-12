# Enriched from F5 iRules reference documentation.
"""FTP::enable -- Enable FTP protocol handler."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FTP__enable.html"


@register
class FtpEnableCommand(CommandDef):
    name = "FTP::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FTP::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable FTP protocol handler.",
                synopsis=("FTP::enable",),
                snippet='Enable FTP protocol handler for FTP message processing. This will enable detection of "AUTH TLS/SSL" for FTP.',
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    FTP::enable\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FTP::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FTP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
