"""iapp::substa -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappSubstaCommand(CommandDef):
    name = "iapp::substa"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::substa",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::substa`.",
                synopsis=("iapp::substa ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::substa ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
