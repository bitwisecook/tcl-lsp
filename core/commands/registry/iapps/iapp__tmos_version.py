"""iapp::tmos_version -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappTmosVersionCommand(CommandDef):
    name = "iapp::tmos_version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::tmos_version",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::tmos_version`.",
                synopsis=("iapp::tmos_version ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::tmos_version ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
