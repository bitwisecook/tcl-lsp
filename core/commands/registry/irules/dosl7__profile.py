# Enriched from F5 iRules reference documentation.
"""DOSL7::profile -- Returns the DOS profile from which the L7-DoS policy is extracted."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__profile.html"


@register
class Dosl7ProfileCommand(CommandDef):
    name = "DOSL7::profile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::profile",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the DOS profile from which the L7-DoS policy is extracted.",
                synopsis=("DOSL7::profile",),
                snippet=(
                    "This command returns the DOS profile from which the L7-DoS policy is\n"
                    "extracted.\n"
                    "Note:\n"
                    "  * in 11.4, default policy returns empty string and if L7-DoS is\n"
                    "    disabled, the <no-profile> string is returned.\n"
                    "  * in 11.5+, default policy returns the one configured with the vip\n"
                    "    and if L7-DoS is disabled, a null string is returned."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::profile",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DOSL7_STATE,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
