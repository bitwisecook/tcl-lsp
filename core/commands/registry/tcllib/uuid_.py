"""uuid -- UUID generation (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib uuid package"
_PACKAGE = "uuid"


@register
class UuidGenerateCommand(CommandDef):
    name = "uuid::uuid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Generate or manipulate UUIDs.",
                synopsis=(
                    "uuid::uuid generate",
                    "uuid::uuid equal id1 id2",
                ),
                snippet="Generate RFC 4122 version 4 universally unique identifiers.",
                source=_SOURCE,
                examples="set id [uuid::uuid generate]",
                return_value="A UUID string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="uuid::uuid subcommand ?args?",
                ),
            ),
            subcommands={
                "equal": SubCommand(
                    name="equal",
                    arity=Arity(2, 2),
                ),
                "generate": SubCommand(
                    name="generate",
                    arity=Arity(0, 0),
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
