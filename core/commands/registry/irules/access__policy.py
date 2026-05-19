# Enriched from F5 iRules reference documentation.
"""ACCESS::policy -- Return information about access policies."""

# Introduced: BIG-IP v11+ (Access Policy Manager) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__policy.html"


_av = make_av(_SOURCE)


@register
class AccessPolicyCommand(CommandDef):
    name = "ACCESS::policy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::policy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return information about access policies.",
                synopsis=(
                    "ACCESS::policy agent_id",
                    "ACCESS::policy evaluate ('-sid' SESSION_ID)",
                    "ACCESS::policy result (-sid SESSION_ID)?",
                    "ACCESS::policy uri",
                ),
                snippet=(
                    "The ACCESS::policy commands allow you to retrieve information about the\n"
                    "access policies in place for a given connection.\n"
                    "\n"
                    "ACCESS::policy agent_id\n"
                    "\n"
                    "     * Returns the identifier for the agent raising the\n"
                    "       ACCESS_POLICY_AGENT_EVENT.\n"
                    "\n"
                    "ACCESS::policy result\n"
                    "\n"
                    "     * Returns back the result of an access policy. The result will be one\n"
                    "       of following:\n"
                    "     * - allow\n"
                    "     * - deny\n"
                    "     * - redirect\n"
                    "\n"
                    "ACCESS::policy uri\n"
                    "\n"
                    "     * Returns TRUE if current request URI is internal to ACCESS (v11+\n"
                    "       only)."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # To avoid clutter, remove the access session for the flow.\n"
                    "    ACCESS::session remove -sid $flow_sid\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::policy <subcommand> ?args?",
                    arg_values={
                        0: (
                            _av("agent_id", "Get the agent identifier.", "ACCESS::policy agent_id"),
                            _av(
                                "evaluate",
                                "Evaluate an access policy.",
                                "ACCESS::policy evaluate ?-sid id?",
                            ),
                            _av(
                                "result",
                                "Get the policy result.",
                                "ACCESS::policy result ?-sid id?",
                            ),
                            _av("uri", "Check if URI is internal to ACCESS.", "ACCESS::policy uri"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "agent_id": SubCommand(
                    name="agent_id",
                    arity=Arity(0, 0),
                    detail="Get the agent identifier.",
                    synopsis="ACCESS::policy agent_id",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "evaluate": SubCommand(
                    name="evaluate",
                    arity=Arity(0),
                    detail="Evaluate an access policy.",
                    synopsis="ACCESS::policy evaluate ?-sid id?",
                    options=(
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                    ),
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "result": SubCommand(
                    name="result",
                    arity=Arity(0),
                    detail="Get the policy result (allow/deny/redirect).",
                    synopsis="ACCESS::policy result ?-sid id?",
                    pure=True,
                    options=(
                        OptionSpec(
                            name="-sid",
                            detail="Session ID.",
                            takes_value=True,
                            value_hint="SESSION_ID",
                        ),
                    ),
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "uri": SubCommand(
                    name="uri",
                    arity=Arity(0, 0),
                    detail="Check if URI is internal to ACCESS.",
                    synopsis="ACCESS::policy uri",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
