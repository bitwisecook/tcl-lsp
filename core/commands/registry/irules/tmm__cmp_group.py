# Enriched from F5 iRules reference documentation.
"""TMM::cmp_group -- Returns the number (0-x) of the group of the CPU executing the rule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TMM__cmp_group.html"


@register
class TmmCmpGroupCommand(CommandDef):
    name = "TMM::cmp_group"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TMM::cmp_group",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number (0-x) of the group of the CPU executing the rule.",
                synopsis=("TMM::cmp_group",),
                snippet=(
                    "This command returns the number (0-x) of the group of the CPU currently\n"
                    "executing the rule. Typically, a group refers to the blade number on a\n"
                    "chassis system, and is always 0 on other platforms. New meanings may be\n"
                    "added for future platform architectures.\n"
                    "This is helpful if you believe one CPU is doing something it shouldn't\n"
                    "and you want to isolate the issue rather than see an aggregate of all\n"
                    "CPUs.\n"
                    "To determine the total number of TMM instances running, see the\n"
                    "TMM::cmp_count page. To determine the CPU ID an iRule is current\n"
                    "executing on within a blade, see the TMM::cmp_unit page."
                ),
                source=_SOURCE,
                examples=(
                    "# Note this example won't work in 10.1.0 - 10.2.2 and 11.0.x\n"
                    "# as the iRule parser doesn't allow these commands in RULE_INIT\n"
                    "when RULE_INIT {\n"
                    "\n"
                    "   # Check if we're running on the first CPU right now\n"
                    "if { [TMM::cmp_unit] == 0 && [TMM::cmp_group] == 0 } {\n"
                    "      # This execution is happening on the first TMM instance\n"
                    "      # Conduct any initialization functionality just once here\n"
                    '      log local0. "some code"\n'
                    "   }\n"
                    "}"
                ),
                return_value="Returns the number (0-x) of the group of the CPU executing the rule.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TMM::cmp_group",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
