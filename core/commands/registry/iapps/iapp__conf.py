"""iapp::conf -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappConfCommand(CommandDef):
    name = "iapp::conf"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::conf",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::conf`.",
                synopsis=("iapp::conf ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::conf ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
