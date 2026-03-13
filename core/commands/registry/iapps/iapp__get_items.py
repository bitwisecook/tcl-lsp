"""iapp::get_items -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappGetItemsCommand(CommandDef):
    name = "iapp::get_items"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::get_items",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::get_items`.",
                synopsis=("iapp::get_items ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::get_items ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
