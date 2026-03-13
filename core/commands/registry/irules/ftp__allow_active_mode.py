# Enriched from F5 iRules reference documentation.
"""FTP::allow_active_mode -- Get or set the state of allow active mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FTP__allow_active_mode.html"


_av = make_av(_SOURCE)


@register
class FtpAllowActiveModeCommand(CommandDef):
    name = "FTP::allow_active_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FTP::allow_active_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the state of allow active mode.",
                synopsis=("FTP::allow_active_mode (enable | disable)?",),
                snippet="Enable or disable active transfer mode. Returns the current status if no option is specified.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                FTP::allow_active_mode disable\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FTP::allow_active_mode (enable | disable)?",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "FTP::allow_active_mode enable",
                                "FTP::allow_active_mode (enable | disable)?",
                            ),
                            _av(
                                "disable",
                                "FTP::allow_active_mode disable",
                                "FTP::allow_active_mode (enable | disable)?",
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
