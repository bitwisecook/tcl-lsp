# Enriched from F5 iRules reference documentation.
"""drop -- Causes the current packet or connection to be dropped/discarded."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/drop.html"


@register
class DropCommand(CommandDef):
    name = "drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the current packet or connection to be dropped/discarded.",
                synopsis=("drop",),
                snippet=(
                    "Causes the current packet or connection (depending on the context of\n"
                    "the event) to be dropped/discarded and the rule continues (no implied\n"
                    "return). This command is identical to discard.\n"
                    "\n"
                    "**Warning**: After `drop`, the current iRule continues executing, and\n"
                    "other iRules and later priorities in this event also run. This can\n"
                    "cause TCL errors (e.g. `ASM::disable` on a dropped connection).\n"
                    "Always follow `drop` with `event disable all` and `return`."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [IP::addr [IP::client_addr] equals 10.1.1.80] } {\n"
                    "    drop\n"
                    "    event disable all\n"
                    "    return\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="drop",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
