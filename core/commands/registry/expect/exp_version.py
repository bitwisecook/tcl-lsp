"""exp_version -- Query or require an Expect version."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exp_version(1)"


@register
class ExpVersionCommand(CommandDef):
    name = "exp_version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exp_version",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Query or require a minimum Expect version.",
                synopsis=("exp_version ?version?",),
                snippet=(
                    "Without arguments, returns the current Expect version. "
                    "With a version argument, raises an error if the running "
                    "Expect is older than the specified version."
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="exp_version ?version?"),),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )
