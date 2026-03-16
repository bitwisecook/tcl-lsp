# Enriched from F5 iRules reference documentation.
"""ACCESS::session -- Access or manipulate session information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__session.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.APM_STATE,
        reads=True,
        connection_side=ConnectionSide.BOTH,
    ),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.APM_STATE,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)


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
                    synopsis="ACCESS::session <subcommand> ?options? ?args?",
                    options=(
                        OptionSpec(
                            name="-flow", detail="Create a flow-scoped session.", takes_value=False
                        ),
                        OptionSpec(
                            name="-timeout",
                            detail="Session timeout in seconds.",
                            takes_value=True,
                            value_hint="SECONDS",
                        ),
                        OptionSpec(
                            name="-lifetime",
                            detail="Session lifetime in seconds.",
                            takes_value=True,
                            value_hint="SECONDS",
                        ),
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                        OptionSpec(name="-remaining", detail="Remaining time.", takes_value=True),
                        OptionSpec(
                            name="-secure", detail="Access secure session data.", takes_value=False
                        ),
                        OptionSpec(
                            name="-config", detail="Access config session data.", takes_value=False
                        ),
                        OptionSpec(
                            name="-ssid",
                            detail="Sub-session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "Create a new session.",
                                "ACCESS::session create ?-flow? ?-timeout secs? ?-lifetime secs?",
                            ),
                            _av(
                                "modify",
                                "Modify an existing session.",
                                "ACCESS::session modify ?-sid id? ?-timeout secs? ?-lifetime secs?",
                            ),
                            _av(
                                "exists",
                                "Check if a session exists.",
                                "ACCESS::session exists ?-sid id?",
                            ),
                            _av(
                                "data",
                                "Get or set session data.",
                                "ACCESS::session data <get|set> ?-sid id? <key> ?value?",
                            ),
                            _av("remove", "Remove a session.", "ACCESS::session remove ?-sid id?"),
                            _av("sid", "Get the session ID.", "ACCESS::session sid"),
                        )
                    },
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
            subcommands={
                "create": SubCommand(
                    name="create",
                    arity=Arity(0),
                    detail="Create a new session.",
                    synopsis="ACCESS::session create ?-flow? ?-timeout secs? ?-lifetime secs?",
                    mutator=True,
                    options=(
                        OptionSpec(
                            name="-flow", detail="Create a flow-scoped session.", takes_value=False
                        ),
                        OptionSpec(
                            name="-timeout",
                            detail="Session timeout in seconds.",
                            takes_value=True,
                            value_hint="SECONDS",
                        ),
                        OptionSpec(
                            name="-lifetime",
                            detail="Session lifetime in seconds.",
                            takes_value=True,
                            value_hint="SECONDS",
                        ),
                    ),
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "modify": SubCommand(
                    name="modify",
                    arity=Arity(0),
                    detail="Modify an existing session.",
                    synopsis="ACCESS::session modify ?-sid id? ?-timeout secs? ?-lifetime secs?",
                    mutator=True,
                    options=(
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                        OptionSpec(
                            name="-timeout",
                            detail="Session timeout in seconds.",
                            takes_value=True,
                            value_hint="SECONDS",
                        ),
                        OptionSpec(
                            name="-lifetime",
                            detail="Session lifetime in seconds.",
                            takes_value=True,
                            value_hint="SECONDS",
                        ),
                        OptionSpec(name="-remaining", detail="Remaining time.", takes_value=True),
                    ),
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(0, 1),
                    detail="Check if a session exists.",
                    synopsis="ACCESS::session exists ?-sid id?",
                    pure=True,
                    options=(
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                        OptionSpec(
                            name="-state_allow", detail="Check for allow state.", takes_value=False
                        ),
                        OptionSpec(
                            name="-state_deny", detail="Check for deny state.", takes_value=False
                        ),
                        OptionSpec(
                            name="-state_redirect",
                            detail="Check for redirect state.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-state_inprogress",
                            detail="Check for in-progress state.",
                            takes_value=False,
                        ),
                    ),
                    side_effect_hints=_READ_EFFECT,
                ),
                "data": SubCommand(
                    name="data",
                    arity=Arity(1),
                    detail="Get or set session data.",
                    synopsis="ACCESS::session data <get|set> ?-sid id? <key> ?value?",
                    arg_values={
                        0: (
                            _av(
                                "get",
                                "Get session variable value.",
                                "ACCESS::session data get ?-sid id? <key>",
                            ),
                            _av(
                                "set",
                                "Set session variable value.",
                                "ACCESS::session data set ?-sid id? <key> <value>",
                            ),
                        )
                    },
                    options=(
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                        OptionSpec(
                            name="-secure", detail="Access secure session data.", takes_value=False
                        ),
                        OptionSpec(
                            name="-config", detail="Access config session data.", takes_value=False
                        ),
                        OptionSpec(
                            name="-ssid",
                            detail="Sub-session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                    ),
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(0),
                    detail="Remove a session.",
                    synopsis="ACCESS::session remove ?-sid id?",
                    mutator=True,
                    options=(
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                    ),
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "sid": SubCommand(
                    name="sid",
                    arity=Arity(0, 0),
                    detail="Get the session ID.",
                    synopsis="ACCESS::session sid",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
            },
        )
