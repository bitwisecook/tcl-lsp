"""iapp::make_safe_password -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappMakeSafePasswordCommand(CommandDef):
    name = "iapp::make_safe_password"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::make_safe_password",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::make_safe_password`.",
                synopsis=("iapp::make_safe_password ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::make_safe_password ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
