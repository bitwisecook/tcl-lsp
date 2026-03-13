"""iapp::upgrade -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappUpgradeCommand(CommandDef):
    name = "iapp::upgrade"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::upgrade",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::upgrade`.",
                synopsis=("iapp::upgrade ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::upgrade ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
