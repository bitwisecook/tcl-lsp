# Enriched from F5 iRules reference documentation.
"""TMM::cmp_unit -- Returns the number (0-x) of the CPU executing the rule."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TMM__cmp_unit.html"


@register
class TmmCmpUnitCommand(CommandDef):
    name = "TMM::cmp_unit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TMM::cmp_unit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number (0-x) of the CPU executing the rule.",
                synopsis=("TMM::cmp_unit",),
                snippet=(
                    "This command returns the number (0-x) of the CPUs executing the rule.\n"
                    "Helpful if you believe one of the CPUs is doing something it shouldn't\n"
                    "and you want to isolate the issue rather than see an aggregate of all\n"
                    "CPUs.\n"
                    "To determine the total number of TMM instances running, see the\n"
                    "TMM::cmp_count page. To determine which blade an iRule is current\n"
                    "executing on, see the TMM::cmp_group page.\n"
                    "\n"
                    "Note that in versions v10.1.0 through v10.2.2 and v11.0.0, this command\n"
                    "is valid in all events except RULE_INIT. This limitation was\n"
                    "removed in v10.2.3 and v11.1.0 (ID 342860)."
                ),
                source=_SOURCE,
                examples=(
                    "# Note this example won't work in 10.1.0 - 10.2.x\n"
                    "# as the iRule parser doesn't allow TMM::cmp_unit in RULE_INIT\n"
                    "when RULE_INIT {\n"
                    "\n"
                    "   # Check if we're running on the first CPU right now\n"
                    "if {[TMM::cmp_unit] == 0}{\n"
                    "      # This execution is happening on the first TMM instance\n"
                    "      # Conduct any initialization functionality just once here\n"
                    '      log local0. "some code"\n'
                    "   }\n"
                    "}"
                ),
                return_value="Returns the number (0-x) of the CPU executing the rule.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TMM::cmp_unit",
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
