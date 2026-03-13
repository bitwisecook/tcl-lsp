# Enriched from F5 iRules reference documentation.
"""FTP::port -- Controls the range of passive mode FTP ephemeral ports."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FTP__port.html"


@register
class FtpPortCommand(CommandDef):
    name = "FTP::port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FTP::port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls the range of passive mode FTP ephemeral ports.",
                synopsis=("FTP::port FIRST (LAST)?",),
                snippet=(
                    "This command allows control over the range of passive mode FTP\n"
                    "ephemeral ports."
                ),
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n  FTP::port 5000 5999\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FTP::port FIRST (LAST)?",
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
