# Enriched from F5 iRules reference documentation.
"""IMAP::activation_mode -- Get or set the activation mode for IMAP STARTTLS."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IMAP__activation_mode.html"


_av = make_av(_SOURCE)


@register
class ImapActivationModeCommand(CommandDef):
    name = "IMAP::activation_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IMAP::activation_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the activation mode for IMAP STARTTLS.",
                synopsis=("IMAP::activation_mode (none | allow | require)?",),
                snippet="Sets the IMAP activation mode to none (IMAP STARTTLS detection will not activate), allow (IMAP will optionally activate TLS if client or server support STARTTLS), or require (IMAP will require that both client and server support STARTTLS). Returns the current activation mode if no option is specified.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    IMAP::activation_mode require\n"
                    "                }\n"
                    "\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    set mode [IMAP::activation_mode]\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IMAP::activation_mode (none | allow | require)?",
                    arg_values={
                        0: (
                            _av(
                                "none",
                                "IMAP::activation_mode none",
                                "IMAP::activation_mode (none | allow | require)?",
                            ),
                            _av(
                                "allow",
                                "IMAP::activation_mode allow",
                                "IMAP::activation_mode (none | allow | require)?",
                            ),
                            _av(
                                "require",
                                "IMAP::activation_mode require",
                                "IMAP::activation_mode (none | allow | require)?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
