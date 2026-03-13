# Enriched from F5 iRules reference documentation.
"""ASM::uncaptcha -- Overrides the CAPTCHA action."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__uncaptcha.html"


@register
class AsmUncaptchaCommand(CommandDef):
    name = "ASM::uncaptcha"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::uncaptcha",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Overrides the CAPTCHA action.",
                synopsis=("ASM::uncaptcha",),
                snippet=(
                    "Overrides the CAPTCHA action for a request mitigated during a Brute-Force attack. \n"
                    "            Consequently, the request will be forwarded to the origin server. \n"
                    "            If the present request was not supposed to be mitigated by CAPTCHA then the command has no effect."
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    "                set i 0\n"
                    "                foreach {viol} [ASM::violation names] {\n"
                    "                    if {$viol eq VIOLATION_ILLEGAL_PARAMETER} {\n"
                    "                        set details [lindex [ASM::violation details] $i]\n"
                    '                        set param_name [b64decode [llookup $details "param_data.param_name"]]\n'
                    "                        #remove the bad parameter from the QS - does not work right in all cases, just for illustration!"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::uncaptcha",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
