# Enriched from F5 iRules reference documentation.
"""ACCESS::perflow -- Returns perflow variable value"""

# Introduced: BIG-IP v11+ (Access Policy Manager) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__perflow.html"


_av = make_av(_SOURCE)

_SETTABLE_VARS = (
    _av("perflow.custom", "Custom perflow variable.", "ACCESS::perflow set perflow.custom <value>"),
    _av(
        "perflow.scratchpad",
        "Scratchpad perflow variable.",
        "ACCESS::perflow set perflow.scratchpad <value>",
    ),
    _av(
        "perflow.custom.flow",
        "Custom flow perflow variable.",
        "ACCESS::perflow set perflow.custom.flow <value>",
    ),
    _av(
        "perflow.scratchpad.flow",
        "Scratchpad flow perflow variable.",
        "ACCESS::perflow set perflow.scratchpad.flow <value>",
    ),
    _av(
        "perflow.l7_protocol_lookup.result",
        "L7 protocol lookup result.",
        "ACCESS::perflow set perflow.l7_protocol_lookup.result <value>",
    ),
)


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
                    synopsis="ACCESS::perflow <get|set> <key> ?value?",
                    arg_values={
                        0: (
                            _av(
                                "get", "Get a perflow variable value.", "ACCESS::perflow get <key>"
                            ),
                            _av(
                                "set",
                                "Set a perflow variable value.",
                                "ACCESS::perflow set <key> <value>",
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
            subcommands={
                "get": SubCommand(
                    name="get",
                    arity=Arity(1, 1),
                    detail="Get a perflow variable value.",
                    synopsis="ACCESS::perflow get <key>",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "set": SubCommand(
                    name="set",
                    arity=Arity(2, 2),
                    detail="Set a perflow variable value.",
                    synopsis="ACCESS::perflow set <key> <value>",
                    mutator=True,
                    arg_values={0: _SETTABLE_VARS},
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.APM_STATE,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
