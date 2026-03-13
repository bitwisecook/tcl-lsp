# Enriched from F5 iRules reference documentation.
"""call -- Calls an iRule procedure."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import ArgRole, Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/call.html"


@register
class CallCommand(CommandDef):
    name = "call"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="call",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Calls an iRule procedure.",
                synopsis=("call <proc_name> [arg(s)]",),
                snippet=(
                    "iRule procedures:\n"
                    "    - Are similar to procedures, functions, subroutines from other languages\n"
                    "    - Allow for reuse of common code\n"
                    "        - Reference the same code from multiple locations, but only define it in one place\n"
                    "        - Simplifies code maintenance\n"
                    "    - Allow you to augment the predefined iRule commands\n"
                    "\n"
                    "Procedures are defined with the proc statement. This must be done\n"
                    "outside of any event. Procedures can be defined within an iRule\n"
                    "assigned to a virtual server or in a separate iRule not assigned to\n"
                    "any virtual server.\n"
                    "\n"
                    "Call a local proc (defined in the same iRule) without a namespace prefix:\n"
                    "    call my_proc $args\n"
                    "\n"
                    "To reference a proc in another iRule in the same partition, prefix with\n"
                    "the iRule name:\n"
                    "    call other_rule::my_proc $args\n"
                    "\n"
                    "To reference a proc in another partition:\n"
                    "    call /other_partition/other_rule::procname args"
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "    # Call a proc which returns no values\n"
                    "    call proc_rule::printArguments one two three\n"
                    "\n"
                    "    # Save the return value of a proc\n"
                    "    set return_values [call proc_rule::returnArguments one two three]\n"
                    "}"
                ),
                return_value="Returns the value(s) that return (if any).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="call <proc_name> [arg(s)]",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_roles={0: ArgRole.NAME},
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
