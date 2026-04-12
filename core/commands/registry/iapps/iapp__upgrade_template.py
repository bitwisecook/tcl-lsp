"""iapp::upgrade_template -- F5 iApps utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IAPPS_ONLY, register

_SOURCE = "F5 iApps utility command list"


@register
class IappUpgradeTemplateCommand(CommandDef):
    name = "iapp::upgrade_template"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="iapp::upgrade_template",
            dialects=_IAPPS_ONLY,
            hover=HoverSnippet(
                summary="F5 iApps utility command `iapp::upgrade_template`.",
                synopsis=("iapp::upgrade_template ?arg ...?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="iapp::upgrade_template ?arg ...?"),),
            validation=ValidationSpec(arity=Arity()),
        )
