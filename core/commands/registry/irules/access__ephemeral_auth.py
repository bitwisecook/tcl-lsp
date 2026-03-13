# Enriched from F5 iRules reference documentation.
"""ACCESS::ephemeral-auth -- Ephemeral auth related iRule"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__ephemeral-auth.html"


@register
class AccessEphemeralAuthCommand(CommandDef):
    name = "ACCESS::ephemeral-auth"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::ephemeral-auth",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Ephemeral auth related iRule",
                synopsis=(
                    "ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?",
                    "ACCESS::ephemeral-auth verify ('-user' USER) ('-password' PASSWORD) ('-protocol' EPHEMERAL_AUTH_PROTOCOL)",
                ),
                snippet=(
                    "Ephemeral auth related iRule\n"
                    "\n"
                    "This command can be used either to create or verify a temporary password for ephemeral authentication.\n"
                    "\n"
                    "ACCESS::ephemeral-auth create [] will create a temporary password and return its value. When auth_cfg is not given, it will use the one deduced from access-config that is associated with the virtual server.  When sid is not given, it will use the one retrieved from the current access environment.\n"
                    "\n"
                    "ACCESS::ephemeral-auth verify [] will verify the user credentials and return the session id that was used to generate temporary password."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    if { [ HTTP::path ] starts_with "/test1" } {\n'
                    "        call ephemeral_auth_test1\n"
                    '        HTTP::respond 200 -content "<html>test1</html>\\n"\n'
                    "    }\n"
                    "}"
                ),
                return_value="ACCESS::ephemeral-auth create [] will return the generated temporary password. ACCESS::ephemeral-auth verify [] will return the session id.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?",
                    options=(
                        OptionSpec(name="-user", detail="Option -user.", takes_value=True),
                        OptionSpec(name="-auth_cfg", detail="Option -auth_cfg.", takes_value=True),
                        OptionSpec(name="-sid", detail="Option -sid.", takes_value=True),
                        OptionSpec(name="-password", detail="Option -password.", takes_value=True),
                        OptionSpec(name="-protocol", detail="Option -protocol.", takes_value=True),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
