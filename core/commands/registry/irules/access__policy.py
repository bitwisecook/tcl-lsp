# Enriched from F5 iRules reference documentation.
"""ACCESS::policy -- Return information about access policies."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__policy.html"


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
                    synopsis="ACCESS::policy agent_id",
                    options=(OptionSpec(name="-sid", detail="Option -sid.", takes_value=True),),
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
        )
