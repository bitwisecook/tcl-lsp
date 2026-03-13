# Enriched from F5 iRules reference documentation.
"""ACCESS::perflow -- Returns perflow variable value"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__perflow.html"


_av = make_av(_SOURCE)


@register
class AccessPerflowCommand(CommandDef):
    name = "ACCESS::perflow"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::perflow",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns perflow variable value",
                synopsis=(
                    "ACCESS::perflow get KEY",
                    "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
                ),
                snippet=(
                    "This command can be used to either set or return the value of a perflow variable that has been set inside the Access Per-Request Policy that is being run.\n"
                    "\n"
                    "            ACCESS::perflow get <var> will return the value of any perflow variable that has already been set. A perflow variable with no value set will return an empty string. An invalid perflow variable name will give a connection reset.\n"
                    "\n"
                    '            ACCESS::perflow set <var> <val> will set the value of the custom perflow variable. Currently the only perflow variables that can be set are "perflow.custom", "perflow.'
                ),
                source=_SOURCE,
                examples=(
                    "when ACCESS_PER_REQUEST_AGENT_EVENT {\n"
                    "                set id [ACCESS::perflow get perflow.irule_agent_id]\n"
                    "\n"
                    '                if { $id eq "irule_agent_one" } {\n'
                    '                    log local0. "Made it to iRule agent in perrequest policy."\n'
                    '                    ACCESS::perflow set perflow.custom "agent_one"\n'
                    "                }\n"
                    "            }"
                ),
                return_value="ACCESS::perflow get will return the string of perflow variable; empty if value isn't set",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::perflow get KEY",
                    arg_values={
                        0: (
                            _av(
                                "perflow.custom",
                                "ACCESS::perflow perflow.custom",
                                "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
                            ),
                            _av(
                                "perflow.scratchpad",
                                "ACCESS::perflow perflow.scratchpad",
                                "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
                            ),
                            _av(
                                "perflow.custom.flow",
                                "ACCESS::perflow perflow.custom.flow",
                                "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
                            ),
                            _av(
                                "perflow.scratchpad.flow",
                                "ACCESS::perflow perflow.scratchpad.flow",
                                "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
                            ),
                            _av(
                                "perflow.l7_protocol_lookup.result",
                                "ACCESS::perflow perflow.l7_protocol_lookup.result",
                                "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
                            ),
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
        )
