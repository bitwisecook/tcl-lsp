"""iapp::destination -- F5 iApps utility command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappDestinationCommand(CommandDef):
    name = "iapp::destination"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::destination",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::destination`.",
                synopsis=("iapp::destination ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::destination ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
