# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::cookie_status -- Returns the status of the Bot Defense cookie."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cookie_status.html"


@register
class BotdefenseCookieStatusCommand(CommandDef):
    name = "BOTDEFENSE::cookie_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::cookie_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the status of the Bot Defense cookie.",
                synopsis=("BOTDEFENSE::cookie_status",),
                snippet=(
                    "Returns the status of the Bot Defense cookie that is received on the request. The returned value is one of the following strings:\n"
                    "    * not_received - the cookie did not appear in the request\n"
                    "    * valid - the cookie is valid and not expired\n"
                    "    * invalid - the cookie cannot be parsed; this could mean that it was modified by an attacker, or that it is older than two days, or due to a configuration change\n"
                    "    * expired - the cookie is valid, but is expired\n"
                    "    * valid_redirect_challenge - the cookie of the redirect was validated\n"
                    "    * renewal - browser challenge answer is about to expire"
                ),
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: In case of an invalid cookie, send a message to High Speed Logging\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    if {[BOTDEFENSE::cookie_status] eq "invalid"} {\n'
                    '        HSL::send $hsl "invalid botdefense cookie from IP [IP::client_addr]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="A string signifying the status of the Bot Defense cookie.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::cookie_status",
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
