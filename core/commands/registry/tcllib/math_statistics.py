"""math::statistics -- Statistical functions (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
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
            validation=ValidationSpec(arity=Arity(2, 3)),
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


def _ms(name: str, summary: str, synopsis: str, arity: Arity, **kw: object) -> type:
    """Register a pure math::statistics command."""

    @register
    class _Cmd(CommandDef):
        pass

    _Cmd.name = f"math::statistics::{name}"
    _Cmd.spec = classmethod(  # type: ignore[assignment]
        lambda cls, _n=f"math::statistics::{name}", _s=summary, _syn=synopsis, _a=arity, _kw=kw: (
            CommandSpec(
                name=_n,
                tcllib_package=_PACKAGE,
                pure=True,
                hover=HoverSnippet(summary=_s, synopsis=(_syn,), source=_SOURCE),
                forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=_syn),),
                validation=ValidationSpec(arity=_a),
                **_kw,  # type: ignore[arg-type]
            )
        )
    )
    return _Cmd


_ms("min", "Return the minimum value.", "math::statistics::min values", Arity(1, 1))
_ms("max", "Return the maximum value.", "math::statistics::max values", Arity(1, 1))
_ms("number", "Return the count of values.", "math::statistics::number values", Arity(1, 1))
_ms("pvar", "Return the population variance.", "math::statistics::pvar values", Arity(1, 1))
_ms(
    "pstdev",
    "Return the population standard deviation.",
    "math::statistics::pstdev values",
    Arity(1, 1),
)
_ms(
    "corr", "Return the correlation coefficient.", "math::statistics::corr data1 data2", Arity(2, 2)
)
_ms(
    "crosscorr",
    "Return the cross-correlation.",
    "math::statistics::crosscorr data1 data2",
    Arity(2, 2),
)
_ms("autocorr", "Return the autocorrelation.", "math::statistics::autocorr data", Arity(1, 1))
_ms(
    "histogram",
    "Compute a histogram.",
    "math::statistics::histogram limits values ?weights?",
    Arity(2, 3),
)
_ms(
    "histogram-alt",
    "Compute a histogram (alternate boundaries).",
    "math::statistics::histogram-alt limits values ?weights?",
    Arity(2, 3),
)
_ms(
    "interval-mean-stdev",
    "Compute confidence interval for mean and stdev.",
    "math::statistics::interval-mean-stdev data confidence",
    Arity(2, 2),
)
_ms(
    "t-test-mean",
    "Perform a t-test on the mean.",
    "math::statistics::t-test-mean data est_mean est_stdev alpha",
    Arity(4, 4),
)
_ms(
    "test-normal",
    "Test for normal distribution.",
    "math::statistics::test-normal data significance",
    Arity(2, 2),
)
_ms(
    "lillieforsFit",
    "Lilliefors goodness-of-fit test.",
    "math::statistics::lillieforsFit values",
    Arity(1, 1),
)
_ms(
    "mean-histogram-limits",
    "Compute histogram limits from mean and stdev.",
    "math::statistics::mean-histogram-limits mean stdev ?number?",
    Arity(2, 3),
)
_ms(
    "minmax-histogram-limits",
    "Compute histogram limits from min and max.",
    "math::statistics::minmax-histogram-limits min max ?number?",
    Arity(2, 3),
)
_ms(
    "linear-model",
    "Fit a linear regression model.",
    "math::statistics::linear-model xdata ydata ?intercept?",
    Arity(2, 3),
)
_ms(
    "linear-residuals",
    "Compute residuals of a linear model.",
    "math::statistics::linear-residuals xdata ydata ?intercept?",
    Arity(2, 3),
)
_ms(
    "test-2x2",
    "Analyse a 2x2 contingency table.",
    "math::statistics::test-2x2 a b c d",
    Arity(4, 4),
)
_ms(
    "print-2x2",
    "Format a 2x2 contingency table.",
    "math::statistics::print-2x2 a b c d",
    Arity(4, 4),
)
_ms(
    "control-xbar",
    "Compute X-bar control chart limits.",
    "math::statistics::control-xbar data ?nsamples?",
    Arity(1, 2),
)
_ms(
    "test-xbar",
    "Test data against X-bar control limits.",
    "math::statistics::test-xbar control data",
    Arity(2, 2),
)
_ms(
    "control-Rchart",
    "Compute R-chart control limits.",
    "math::statistics::control-Rchart data ?nsamples?",
    Arity(1, 2),
)
_ms(
    "test-Rchart",
    "Test data against R-chart control limits.",
    "math::statistics::test-Rchart control data",
    Arity(2, 2),
)
_ms(
    "test-Duckworth",
    "Perform Duckworth test.",
    "math::statistics::test-Duckworth list1 list2 significance",
    Arity(3, 3),
)
_ms("test-anova-F", "One-way ANOVA F-test.", "math::statistics::test-anova-F alpha args", Arity(1))
_ms(
    "test-Tukey-range",
    "Tukey range test.",
    "math::statistics::test-Tukey-range alpha args",
    Arity(1),
)
_ms(
    "test-Dunnett",
    "Dunnett multiple comparison test.",
    "math::statistics::test-Dunnett alpha control args",
    Arity(2),
)
_ms(
    "test-Kruskal-Wallis",
    "Kruskal-Wallis rank test.",
    "math::statistics::test-Kruskal-Wallis confidence args",
    Arity(1),
)
_ms(
    "analyse-Kruskal-Wallis",
    "Analyse Kruskal-Wallis results.",
    "math::statistics::analyse-Kruskal-Wallis args",
    Arity(0),
)
_ms("group-rank", "Rank grouped data.", "math::statistics::group-rank args", Arity(0))
_ms(
    "test-Wilcoxon",
    "Wilcoxon signed-rank test.",
    "math::statistics::test-Wilcoxon sample_a sample_b",
    Arity(2, 2),
)
_ms(
    "spearman-rank",
    "Spearman rank correlation.",
    "math::statistics::spearman-rank sample_a sample_b",
    Arity(2, 2),
)
_ms(
    "spearman-rank-extended",
    "Extended Spearman rank correlation.",
    "math::statistics::spearman-rank-extended sample_a sample_b",
    Arity(2, 2),
)
_ms(
    "filter",
    "Filter data by expression.",
    "math::statistics::filter varname data expression",
    Arity(3, 3),
    arg_roles={0: frozenset({ArgRole.VAR_WRITE}), 2: frozenset({ArgRole.EXPR})},
)
_ms(
    "map",
    "Map data by expression.",
    "math::statistics::map varname data expression",
    Arity(3, 3),
    arg_roles={0: frozenset({ArgRole.VAR_WRITE}), 2: frozenset({ArgRole.EXPR})},
)
_ms(
    "samplescount",
    "Count samples matching expression.",
    "math::statistics::samplescount varname list ?expression?",
    Arity(2, 3),
    arg_roles={0: frozenset({ArgRole.VAR_WRITE}), 2: frozenset({ArgRole.EXPR})},
)
