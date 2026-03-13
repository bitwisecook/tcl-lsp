# Generated from F5 iRules reference documentation -- do not edit manually.
"""TMM::cmp_primary_group -- F5 iRules command `TMM::cmp_primary_group`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TMM__cmp_primary_group.html"


@register
class TmmCmpPrimaryGroupCommand(CommandDef):
    name = "TMM::cmp_primary_group"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TMM::cmp_primary_group",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `TMM::cmp_primary_group`.",
                synopsis=("TMM::cmp_primary_group",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TMM::cmp_primary_group",
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
