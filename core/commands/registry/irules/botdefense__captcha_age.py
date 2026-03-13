# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::captcha_age -- Returns the age of the CAPTCHA challenge in seconds."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__captcha_age.html"


@register
class BotdefenseCaptchaAgeCommand(CommandDef):
    name = "BOTDEFENSE::captcha_age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::captcha_age",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the age of the CAPTCHA challenge in seconds.",
                synopsis=("BOTDEFENSE::captcha_age",),
                snippet='Returns the age of the CAPTCHA challenge in seconds. This is only relevant if the value of BOTDEFENSE::captcha_status is "correct", "renewal" or "expired"; otherwise, -1 is returned.',
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Send CAPTCHA challenge and validate the response, but to avoid\n"
                    "# blocking requests to which CAPTCHA challenge cannot be sent (non-HTML pages),\n"
                    "# send the CAPTCHA challenge on HTML pages after 30 seconds of aging, which is\n"
                    "# before the expiration of the answer.\n"
                    "when BOTDEFENSE_ACTION {\n"
                    '    if {[BOTDEFENSE::action] eq "allow"} {\n'
                    '        if {[BOTDEFENSE::captcha_status] eq "correct"} {\n'
                    "            if {    ([BOTDEFENSE::cs_allowed]) &&"
                ),
                return_value="Returns the age of the CAPTCHA challenge in seconds, or -1 if not applicable.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::captcha_age",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
