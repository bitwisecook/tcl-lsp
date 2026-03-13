# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::cs_allowed -- Returns or sets whether it is allowed for Bot Defense to take a client-side action."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_allowed.html"


@register
class BotdefenseCsAllowedCommand(CommandDef):
    name = "BOTDEFENSE::cs_allowed"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::cs_allowed",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets whether it is allowed for Bot Defense to take a client-side action.",
                synopsis=("BOTDEFENSE::cs_allowed (BOOLEAN)?",),
                snippet=(
                    "Returns or sets if the client-side actions (browser challenge or CAPTCHA challenge or allow_and_browser_challenge_in_response or Device-ID) are allowed on this request. By default, only requests to URLs which are detected as HTML URLs are cs_allowed (qualified), but this command can override this detection.\n"
                    "\n"
                    "Using this command during BOTDEFENSE_ACTION event to modify the value will have no effect and will be ignored.\n"
                    "\n"
                    "Using this command during BOTDEFENSE_REQUEST or HTTP_REQUEST event to modify the value will override the detection."
                ),
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Always allow client-side actions to be taken on URLs with the .html extension.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    if {[HTTP::uri] ends_with ".html"} {\n'
                    "        BOTDEFENSE::cs_allowed true\n"
                    "    }\n"
                    "}"
                ),
                return_value="* When called without any arguments: Returns whether a client-side action is allowed to be taken by Bot Defense. If the value was overridden, the returned value is the overridden one. * When called with an argument for overriding the value of cs_allowed, no value is returned.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::cs_allowed (BOOLEAN)?",
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
