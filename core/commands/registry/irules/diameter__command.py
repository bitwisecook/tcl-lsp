# Enriched from F5 iRules reference documentation.
"""DIAMETER::command -- Gets or sets the command-code."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__command.html"


@register
class DiameterCommandCommand(CommandDef):
    name = "DIAMETER::command"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::command",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the command-code.",
                synopsis=("DIAMETER::command (DIAMETER_COMMAND_CODE)?",),
                snippet="The DIAMETER::command gets or sets the command code in the Diameter message header.",
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER command, with code [DIAMETER::command]"\n'
                    "}"
                ),
                return_value="If new command-code value is not provided, returns command code of current Diameter message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::command (DIAMETER_COMMAND_CODE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
