# Enriched from F5 iRules reference documentation.
"""ASM::policy -- Returns the name of the ASM security policy that was applied for the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__policy.html"


@register
class AsmPolicyCommand(CommandDef):
    name = "ASM::policy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::policy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name of the ASM security policy that was applied for the request.",
                synopsis=("ASM::policy",),
                snippet="Returns the name of the ASM policy that was applied on the request. It can be used to detect which CPM rules are applied or ASM::enable commands are applied on a request.",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_BLOCKING{\n"
                    '    log local0. "The request was blocked using the [ASM::policy] policy"\n'
                    "}"
                ),
                return_value="Returns the ASM policy applied on the request or null string if ASM is disabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::policy",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
