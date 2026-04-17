# Enriched from F5 iRules reference documentation.
"""matchclass -- Performs comparison against the contents of data group."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .class_ import ClassCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/matchclass.html"


@register
class MatchclassCommand(CommandDef):
    name = "matchclass"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="matchclass",
            deprecated_replacement=ClassCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Performs comparison against the contents of data group.",
                synopsis=("matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS",),
                snippet=(
                    "Performs comparisons against the contents of data group. Typically used\n"
                    "for conditional logic control.\n"
                    "\n"
                    "Note: matchclass has been deprecated in v10 in favor of the new\n"
                    "class commands. The class command offers better functionality and\n"
                    "performance than matchclass.\n"
                    "\n"
                    "Note that you should not use a $:: or :: prefix on the datagroup name\n"
                    "when using the matchclass command (or in any datagroup reference on\n"
                    "9.4.4 or later).\n"
                    "\n"
                    "In v9.4.4 - 10, using $::datagroup_name will work but demote the\n"
                    "virtual server from running on all TMMs. For details, see the CMP\n"
                    "compatibility page."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [matchclass [IP::remote_addr] equals aol] } {\n"
                    "     pool aol_pool\n"
                    "  } else {\n"
                    "     pool all_pool\n"
                    " }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DATA_GROUP,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
