# Enriched from F5 iRules reference documentation.
"""AUTH::wantcredential_prompt_style -- Returns an authorization session authidXs credential prompt style."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_prompt_style.html"


@register
class AuthWantcredentialPromptStyleCommand(CommandDef):
    name = "AUTH::wantcredential_prompt_style"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::wantcredential_prompt_style",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an authorization session authidXs credential prompt style.",
                synopsis=("AUTH::wantcredential_prompt_style AUTH_ID",),
                snippet=(
                    "Returns the authorization session authid’s credential prompt style that\n"
                    "the system last requested (when the system generated an\n"
                    "AUTH_WANTCREDENTIAL event). The value of the <authid> argument is\n"
                    "either echo_on, echo_off, or unknown. This command is especially\n"
                    "helpful in providing authentication services to interactive protocols\n"
                    "(or example, telnet and ftp), where the actual text prompts and\n"
                    "responses may be directly communicated with the remote user."
                ),
                source=_SOURCE,
                examples=(
                    "when AUTH_WANTCREDENTIAL {\n"
                    '  HTTP::respond 401 "WWW-Authenticate" "Basic realm=\\"\\""\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::wantcredential_prompt_style AUTH_ID",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
