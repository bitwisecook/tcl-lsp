"""math::statistics -- Statistical functions (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib math::statistics package"
_PACKAGE = "math::statistics"


@register
class MathStatMeanCommand(CommandDef):
    name = "math::statistics::mean"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute the arithmetic mean of a list of values.",
                synopsis=("math::statistics::mean data",),
                source=_SOURCE,
                return_value="The arithmetic mean.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="math::statistics::mean data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class MathStatStdevCommand(CommandDef):
    name = "math::statistics::stdev"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute the standard deviation of a list of values.",
                synopsis=("math::statistics::stdev data",),
                source=_SOURCE,
                return_value="The standard deviation.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="math::statistics::stdev data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class MathStatVarianceCommand(CommandDef):
    name = "math::statistics::var"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute the variance of a list of values.",
                synopsis=("math::statistics::var data",),
                source=_SOURCE,
                return_value="The variance.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="math::statistics::var data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class MathStatMedianCommand(CommandDef):
    name = "math::statistics::median"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute the median of a list of values.",
                synopsis=("math::statistics::median data",),
                source=_SOURCE,
                return_value="The median value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="math::statistics::median data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class MathStatQuantilesCommand(CommandDef):
    name = "math::statistics::quantiles"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute quantiles of a list of values.",
                synopsis=("math::statistics::quantiles data confidences",),
                source=_SOURCE,
                return_value="A list of quantile values.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="math::statistics::quantiles data confidences",
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
            pure=True,
        )


@register
class MathStatBasicStatsCommand(CommandDef):
    name = "math::statistics::basic-stats"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute basic statistics (mean, min, max, count, stdev).",
                synopsis=("math::statistics::basic-stats data",),
                source=_SOURCE,
                return_value=("A list: mean min max count stdev variance."),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="math::statistics::basic-stats data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )
