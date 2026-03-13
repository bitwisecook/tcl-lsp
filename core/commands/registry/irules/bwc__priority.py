# Enriched from F5 iRules reference documentation.
"""BWC::priority -- This command is used to create a priority class for a bwc policy."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__priority.html"


@register
class BwcPriorityCommand(CommandDef):
    name = "BWC::priority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::priority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used to create a priority class for a bwc policy.",
                synopsis=("BWC::priority (PRIORITY_CLASS WEIGHT)",),
                snippet=(
                    "A BWC policy instance or category can be mapped to a priority class of a priority group. This is part of the configuration and can be done via tmsh or GUI. Once a BWC instance has these mappings we can use the iRule defined below to change those. The syntax for this iRule is like below,\n"
                    "\n"
                    "BWC::priority <name1> <value1> [<name2> <value2>] [<name3> <value3>] [<name4> <value4>]\n"
                    "\n"
                    "    nameX - name of a priority class. valueX - weight of the priority class in percentage.\n"
                    "\n"
                    "In the above iRule you can specify one or more traffic classes and specify the desired weight-percentage for that priority-class."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "\n"
                    "    set my_policy bwc_policy set my_cat none set my_cookie [IP::remote_addr] switch -glob [TCP::remote_port] {\n"
                    '        "101" {\n'
                    "            set my_cat c1\n"
                    "        }\n"
                    '        "102" {\n'
                    "            set my_cat c2\n"
                    "        }\n"
                    "    }\n"
                    '    BWC::policy attach $my_policy $my_cookie if { $my_cat != "none" } {\n'
                    '        log local0. "BWC::color set $my_policy $my_cat" BWC::color set $my_policy $my_cat BWC::priority "tc1" 60 "tc2" 40\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::priority (PRIORITY_CLASS WEIGHT)",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
