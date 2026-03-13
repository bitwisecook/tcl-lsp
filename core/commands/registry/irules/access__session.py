# Enriched from F5 iRules reference documentation.
"""ACCESS::session -- Access or manipulate session information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__session.html"


@register
class AccessSessionCommand(CommandDef):
    name = "ACCESS::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Access or manipulate session information.",
                synopsis=(
                    "ACCESS::session create (('-flow')? ('-timeout' TIMEOUT)? ('-lifetime' LIFETIME)?)#",
                    "ACCESS::session modify ('-sid' SESSION_ID)? (('-timeout' TIMEOUT)? (('-lifetime' LIFETIME)? | ('-remaining' REMAINING)?))#",
                    "ACCESS::session exists ('-state_allow' | '-state_deny' | '-state_redirect' | '-state_inprogress')? (-sid)? (SESSION_ID)?",
                    "ACCESS::session data get ('-sid' SESSION_ID)? ('-secure' | '-config')? KEY (-ssid SESSION_ID)?",
                ),
                snippet=(
                    "The different permutations of the ACCESS::session command allow you to\n"
                    "access or manipulate different portions of session information when\n"
                    "dealing with APM requests.\n"
                    "\n"
                    "ACCESS::session data get\n"
                    "\n"
                    "     * Returns the value of session variable.\n"
                    "\n"
                    "ACCESS::session data set [ ]\n"
                    "\n"
                    "     * Sets the value of session variable to be the given.\n"
                    "\n"
                    "ACCESS::session exists\n"
                    "\n"
                    "     * This commands returns TRUE when the session with provided sid\n"
                    "       exists, and returns FALSE otherwise. This command is allowed to be\n"
                    "       executed in different events other then ACCESS events. This command\n"
                    "       added in version 10."
                ),
                source=_SOURCE,
                examples=(
                    "when ACCESS_ACL_ALLOWED {\n"
                    'set user [ACCESS::session data get "session.logon.last.username"]\n'
                    'HTTP::header insert "X-USERNAME" $user\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::session create (('-flow')? ('-timeout' TIMEOUT)? ('-lifetime' LIFETIME)?)#",
                    options=(
                        OptionSpec(name="-flow", detail="Option -flow.", takes_value=True),
                        OptionSpec(name="-timeout", detail="Option -timeout.", takes_value=True),
                        OptionSpec(name="-lifetime", detail="Option -lifetime.", takes_value=True),
                        OptionSpec(name="-sid", detail="Option -sid.", takes_value=True),
                        OptionSpec(
                            name="-remaining", detail="Option -remaining.", takes_value=True
                        ),
                        OptionSpec(
                            name="-state_allow", detail="Option -state_allow.", takes_value=True
                        ),
                        OptionSpec(
                            name="-state_deny", detail="Option -state_deny.", takes_value=True
                        ),
                        OptionSpec(
                            name="-state_redirect",
                            detail="Option -state_redirect.",
                            takes_value=True,
                        ),
                        OptionSpec(
                            name="-state_inprogress",
                            detail="Option -state_inprogress.",
                            takes_value=True,
                        ),
                        OptionSpec(name="-secure", detail="Option -secure.", takes_value=True),
                        OptionSpec(name="-config", detail="Option -config.", takes_value=True),
                        OptionSpec(name="-ssid", detail="Option -ssid.", takes_value=True),
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
