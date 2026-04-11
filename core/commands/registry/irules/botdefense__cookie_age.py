# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::cookie_age -- Returns the age of the Bot Defense cookie in seconds."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cookie_age.html"


@register
class BotdefenseCookieAgeCommand(CommandDef):
    name = "BOTDEFENSE::cookie_age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::cookie_age",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the age of the Bot Defense cookie in seconds.",
                synopsis=("BOTDEFENSE::cookie_age",),
                snippet=(
                    'Returns the age of the Bot Defense browser cookie in seconds. This is only relevant if the value of BOTDEFENSE::cookie_status is either "valid", "expired" or "renewal"; otherwise, -1 is returned.\n'
                    "\n"
                    "Note that In the previous version the returned status referred to both device_id and browser challenge, but now it only returns the age of the browser challenge."
                ),
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: In case of an expired cookie, log the age of the cookie\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    if {[BOTDEFENSE::cookie_status] eq "expired"} {\n'
                    '        set log "expired botdefense cookie (from [BOTDEFENSE::cookie_age]"\n'
                    '        append log " seconds ago) from IP [IP::client_addr]"\n'
                    "        HSL::send $hsl $log\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the age of the Bot Defense cookie in seconds, or -1 if not applicable.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::cookie_age",
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
