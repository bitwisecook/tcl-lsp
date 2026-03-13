# Enriched from F5 iRules reference documentation.
"""FTP::ftps_mode -- Get or set the activation mode for FTPS."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FTP__ftps_mode.html"


_av = make_av(_SOURCE)


@register
class FtpFtpsModeCommand(CommandDef):
    name = "FTP::ftps_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FTP::ftps_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the activation mode for FTPS.",
                synopsis=("FTP::ftps_mode (disallow | allow | require)?",),
                snippet='Sets the FTPS activation mode to disallow (FTP commands "AUTH SSL/TLS" will be filtered out, and implicit FTPS connection will be dropped), allow (FTP will optionally activate TLS if client or server support "AUTH SSL/TLS"), or require (FTP will require that the client and server complete "AUTH SSL/TLS" before data transfers).',
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    FTP::ftps_mode require\n"
                    "                }\n"
                    "\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    set mode [FTP::ftps_mode]\n"
                    "                }\n"
                    "            }"
                ),
                return_value="Returns the current activation mode.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FTP::ftps_mode (disallow | allow | require)?",
                    arg_values={
                        0: (
                            _av(
                                "disallow",
                                "FTP::ftps_mode disallow",
                                "FTP::ftps_mode (disallow | allow | require)?",
                            ),
                            _av(
                                "allow",
                                "FTP::ftps_mode allow",
                                "FTP::ftps_mode (disallow | allow | require)?",
                            ),
                            _av(
                                "require",
                                "FTP::ftps_mode require",
                                "FTP::ftps_mode (disallow | allow | require)?",
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
                    target=SideEffectTarget.FTP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
