# Enriched from F5 iRules reference documentation.
"""after -- Execute iRules code after a set period of delay."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/after.html"


@register
class AfterCommand(CommandDef):
    name = "after"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="after",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Execute iRules code after a set period of delay.",
                synopsis=(
                    "after MILLI_SECONDS (-periodic)? (NESTING_SCRIPT)?",
                    "after cancel (-current | (ID)+)",
                    "after info (ID)*",
                ),
                snippet=(
                    "The after command allows you to insert a delay into the processing of your iRule, executing the specified script after a certain amount of time has passed. It also allows for things like periodic (repeat) execution of a script, as well as looking up or canceling currently delayed scripts.\n"
                    "\n"
                    "Note: The after command is not available in GTM.\n"
                    "    \n"
                    "Delays rule execution for the given milliseconds. If SCRIPT parameter is given, schedules it for execution after the given milliseconds.  If -periodic switch is supplied, the script will be evaluated every given milliseconds."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "   set users_last_sec 0\n"
                    "   set new_user_count 0\n"
                    "   after 1000 ¨Cperiodic {\n"
                    "      set users_last_sec $new_user_count\n"
                    "      set new_user_count 0\n"
                    "   }\n"
                    "}"
                ),
                return_value="When script is named, an id is returned for the script.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="after MILLI_SECONDS (-periodic)? (NESTING_SCRIPT)?",
                    options=(
                        OptionSpec(name="-periodic", detail="Option -periodic.", takes_value=False),
                        OptionSpec(name="-current", detail="Option -current.", takes_value=False),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            xc_translatable=False,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
