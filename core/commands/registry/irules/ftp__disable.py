# Enriched from F5 iRules reference documentation.
"""FTP::disable -- Disable FTP protocol handler."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FTP__disable.html"


@register
class FtpDisableCommand(CommandDef):
    name = "FTP::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FTP::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable FTP protocol handler.",
                synopsis=("FTP::disable",),
                snippet='Disable FTP protocol handler for FTP message processing. This will disable detection of "AUTH TLS/SSL" for FTP.',
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    FTP::disable\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FTP::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FTP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
