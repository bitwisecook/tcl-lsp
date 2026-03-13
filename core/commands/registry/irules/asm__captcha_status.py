# Enriched from F5 iRules reference documentation.
"""ASM::captcha_status -- Returns the status of the user's answer to the CAPTCHA challenge."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__captcha_status.html"


@register
class AsmCaptchaStatusCommand(CommandDef):
    name = "ASM::captcha_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::captcha_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the status of the user's answer to the CAPTCHA challenge.",
                synopsis=("ASM::captcha_status",),
                snippet=(
                    "Returns the status of the user's answer to the CAPTCHA challenge. The returned value is one of the following strings:\n"
                    "            not_received - the answer to the CAPTCHA challenge did not appear in the request; this is the normal result, before the CAPTCHA challenge is sent to the client\n"
                    "            correct - the answer is correct\n"
                    "            incorrect - the answer is incorrect\n"
                    "            empty - an empty answer was given, or if the user clicked on the CAPTCHA Refresh button"
                ),
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Send a CAPTCHA challenge on the login page, and only allow the\n"
                    "            # login if the user passed the CAPTCHA challenge\n"
                    "            when ASM_REQUEST_DONE {\n"
                    '                if {[ASM::captcha_status] ne "correct"} {\n'
                    '                    if {[HTTP::uri] eq "/t/login.php"} {\n'
                    "                        set res [ASM::captcha]\n"
                    '                        if {$res ne "ok"} {\n'
                    '                            log local0. "cannot send captcha_challenge: \\"$res\\""\n'
                    "                        }"
                ),
                return_value="Returns a string signifying the status of the CAPTCHA challenge.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::captcha_status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
