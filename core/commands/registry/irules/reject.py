# Enriched from F5 iRules reference documentation.
"""reject -- Causes the connection to be rejected."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/reject.html"


@register
class RejectCommand(CommandDef):
    name = "reject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="reject",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the connection to be rejected.",
                synopsis=("reject",),
                snippet=(
                    "Causes the connection to be rejected, returning a reset as appropriate\n"
                    "for the protocol. Subsequent code in the current event in the current\n"
                    "iRule or other iRules on the VS are still executed prior to the reset\n"
                    "being sent.\n"
                    "\n"
                    "**Warning**: After `reject`, the current iRule continues executing,\n"
                    "and other iRules on the VS also run. This can cause TCL errors\n"
                    "(e.g. `ASM::disable` on a rejected connection). Always follow\n"
                    "`reject` with `event disable all` and `return`.\n"
                    "\n"
                    "If the VS is using FastHTTP, reject commands will not work, at least\n"
                    "under 11.3.0."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [TCP::local_port] != 443 } {\n"
                    "    reject\n"
                    "    event disable all\n"
                    "    return\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="reject",
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
