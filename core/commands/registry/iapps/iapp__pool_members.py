"""iapp::pool_members -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappPoolMembersCommand(CommandDef):
    name = "iapp::pool_members"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::pool_members",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::pool_members`.",
                synopsis=("iapp::pool_members ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::pool_members ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
