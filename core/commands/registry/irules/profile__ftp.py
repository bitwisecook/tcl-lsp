# Enriched from F5 iRules reference documentation.
"""PROFILE::ftp -- Returns the value of an FTP profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__ftp.html"


@register
class ProfileFtpCommand(CommandDef):
    name = "PROFILE::ftp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::ftp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of an FTP profile setting.",
                synopsis=("PROFILE::ftp ATTR",),
                snippet="Returns the current value of the specified setting in the assigned FTP profile.",
                source=_SOURCE,
                return_value="Returns the current value of the specified setting in the assigned FTP profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::ftp ATTR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
