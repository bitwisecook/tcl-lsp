# Generated from F5 iRules reference documentation -- do not edit manually.
"""DIAG::test -- F5 iRules command `DIAG::test`."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAG__test.html"


@register
class DiagTestCommand(CommandDef):
    name = "DIAG::test"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAG::test",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `DIAG::test`.",
                synopsis=("DIAG::test",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAG::test",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
