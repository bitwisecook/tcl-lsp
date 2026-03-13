# Enriched from F5 iRules reference documentation.
"""PSM::FTP::disable -- To disable PSM for FTP traffic."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSM__FTP__disable.html"


@register
class PsmFtpDisableCommand(CommandDef):
    name = "PSM::FTP::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSM::FTP::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="To disable PSM for FTP traffic.",
                synopsis=("PSM::FTP::disable",),
                snippet="To disable PSM for FTP traffic",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSM::FTP::disable",
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
