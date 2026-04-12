# Enriched from F5 iRules reference documentation.
"""ASM::captcha -- Responds to the client with a CAPTCHA challenge."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__captcha.html"


@register
class AsmCaptchaCommand(CommandDef):
    name = "ASM::captcha"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::captcha",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Responds to the client with a CAPTCHA challenge.",
                synopsis=("ASM::captcha",),
                snippet=(
                    "Responds to the client with a CAPTCHA challenge. \n"
                    "            Note although ASM will send the CAPTCHA challenge screen back to the user, the enforcement is not always done automatically. \n"
                    "            To enforce the correct CAPTCHA response, the ASM::captcha_status command should be used."
                ),
                source=_SOURCE,
                examples=(
                    "le counts the number of violations, and if it exceeds 3,\n"
                    "            # it issues a CAPTCHA action.\n"
                    "            when ASM_REQUEST_DONE {\n"
                    '                if {[ASM::violation count] > 3 and [ASM::severity] eq "Error"} {\n'
                    "                    ASM::captcha\n"
                    "                }\n"
                    "            }"
                ),
                return_value='Returns a string signifying if the challenge was sent successfully: "ok" - CAPTCHA challenge was sent successfully "nok asm blocked request" - CAPTCHA challenge was not sent, because a blocking page action was performed "nok asm uncaptcha command was raised" - CAPTCHA challenge was not sent, because…',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::captcha",
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
