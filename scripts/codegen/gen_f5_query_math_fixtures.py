#!/usr/bin/env python3
"""Golden fixtures for the Rust query *math-category builtins*.

For a list of `query` strings (each over a `null` JSON root, mode ``json``)
runs the Python query DSL end to end — parse → evaluate → ``output.render``
— and records the rendered string (or the ``error:`` message). The Rust
``math_parity`` test mirrors the same pipeline and asserts byte-equality, so
the math builtins stay byte-faithful to the Python source. Self-contained:
the fixtures are captured once here and checked in.

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_math_fixtures.py
"""

from __future__ import annotations

import json
from pathlib import Path

from dialects.f5.bigip.model import BigipConfig
from dialects.f5.query.errors import QueryError
from dialects.f5.query.evaluator import EvalContext, evaluate
from dialects.f5.query.output import render
from dialects.f5.query.parser import parse_query
from dialects.f5.query.source_map import SourceMap
from dialects.f5.query.values import Root


def run(query: str, data: object, mode: str) -> dict:
    root = Root(
        uri="data.json",
        source="",
        config=BigipConfig(),
        source_map=SourceMap.build(""),
        json_value=data,
    )
    ctx = EvalContext(root=root)
    try:
        prog = parse_query(query)
        values = evaluate(prog, ctx)
        out = render(values, mode=mode)
        return {"query": query, "input": data, "mode": mode, "kind": "ok", "output": out}
    except (QueryError, ValueError) as exc:
        return {"query": query, "input": data, "mode": mode, "kind": "err", "output": str(exc)}


# Every math builtin ported into `builtins/math.rs`, with representative
# inputs: integer vs float, negatives, domain errors, the 2-/3-arg forms and
# the list-returning ones. Root is always `null` (`None`); each query stands
# alone. Mode is "json".
QUERIES: list[str] = [
    # round / sign / truncation
    "round(2.5)",
    "round(-2.5)",
    "round(2.49)",
    "round(2.51)",
    "round(0.5)",
    "round(-0.5)",
    "round(3)",
    "fabs(-5)",
    "fabs(3.14)",
    "fabs(-2.5)",
    "trunc(3.9)",
    "trunc(-3.9)",
    "trunc(3)",
    "rint(2.5)",
    "rint(3.5)",
    "rint(-2.5)",
    "rint(2.49)",
    "rint(3)",
    "nearbyint(2.5)",
    "nearbyint(3.5)",
    "nearbyint(-0.5)",
    # powers / roots / logs
    "sqrt(16)",
    "sqrt(2)",
    "sqrt(0)",
    "sqrt(-1)",
    "sqrt(-2.5)",
    "cbrt(27)",
    "cbrt(-8)",
    "cbrt(2)",
    "cbrt(0)",
    "pow(2, 10)",
    "pow(2, 0.5)",
    "pow(2.0, -1)",
    "pow(0, 0)",
    "exp(1)",
    "exp(0)",
    "exp(-1)",
    "exp2(10)",
    "exp2(0.5)",
    "exp10(3)",
    "exp10(0)",
    "pow10(3)",
    "pow10(-2)",
    "expm1(0)",
    "expm1(1)",
    "log(2.718281828459045)",
    "log(1)",
    "log(0)",
    "log(-1)",
    "log10(1000)",
    "log10(0)",
    "log2(1024)",
    "log2(0)",
    "log1p(0)",
    "log1p(1)",
    "logb(8)",
    "logb(0.25)",
    "logb(0)",
    "logb(-8)",
    "logb(infinite)",
    "significand(12)",
    "significand(-12)",
    "significand(0)",
    "significand(1)",
    # constants / classification
    "nan",
    "infinite",
    "-infinite",
    "isnan(nan)",
    "isnan(1.0)",
    "isnan(1)",
    'isnan("x")',
    "isinfinite(infinite)",
    "isinfinite(-infinite)",
    "isinfinite(1.0)",
    "isinfinite(1)",
    "isnormal(1.0)",
    "isnormal(0)",
    "isnormal(0.0)",
    "isnormal(nan)",
    "isnormal(infinite)",
    "isnormal(5)",
    'isnormal("x")',
    # trig / hyperbolic
    "sin(0)",
    "cos(0)",
    "tan(0)",
    "asin(0.5)",
    "acos(1)",
    "acos(2)",
    "asin(2)",
    "atan(1)",
    "atan2(1, 1)",
    "atan2(0, -1)",
    "sinh(1)",
    "cosh(1)",
    "tanh(1)",
    "asinh(1)",
    "acosh(1)",
    "acosh(0.5)",
    "atanh(0.5)",
    "atanh(2)",
    # IEEE helpers
    "copysign(3, -1)",
    "copysign(-3, 1)",
    "copysign(3.5, -2)",
    "fdim(5, 3)",
    "fdim(3, 5)",
    "fdim(3.5, 1.0)",
    "frexp(12)",
    "frexp(0)",
    "frexp(-12)",
    "frexp(0.5)",
    "ldexp(0.75, 4)",
    "ldexp(3, 2)",
    "ldexp(1, -2)",
    "modf(3.75)",
    "modf(-3.75)",
    "modf(5)",
    "hypot(3, 4)",
    "hypot(5, 12)",
    "hypot(0, 0)",
    "fma(2, 3, 1)",
    "fma(2.5, 4, -3)",
    "fmax(3, 7)",
    "fmax(7, 3)",
    "fmax(3.0, 7)",
    "fmax(3.5, 3.5)",
    "fmin(3, 7)",
    "fmin(7, 3)",
    "fmin(3.0, 7)",
    "fmod(7, 3)",
    "fmod(-7, 3)",
    "fmod(7.5, 2)",
    "remainder(7, 3)",
    "remainder(5, 3)",
    "remainder(-7, 3)",
    "drem(7, 3)",
    "drem(5, 3)",
    # gamma family
    "gamma(5)",
    "gamma(0.5)",
    "gamma(-1)",
    "gamma(-0.5)",
    "tgamma(5)",
    "tgamma(0.5)",
    "lgamma(5)",
    "lgamma(0.5)",
    "lgamma(-2.5)",
    "lgamma_r(5)",
    "lgamma_r(0.5)",
    "lgamma_r(-2.5)",
    "lgamma_r(-0.5)",
    # bessel
    "j0(0)",
    "j0(1)",
    "j0(5)",
    "j0(10)",
    "j1(0)",
    "j1(1)",
    "j1(5)",
    "j1(10)",
    "y0(1)",
    "y0(5)",
    "y0(10)",
    "y0(0)",
    "y1(1)",
    "y1(5)",
    "y1(10)",
    "y1(-1)",
    "jn(2, 5)",
    "jn(3, 10)",
    "jn(0, 5)",
    "jn(1, 5)",
    "jn(-2, 5)",
    "jn(2, 0)",
    "yn(2, 5)",
    "yn(3, 10)",
    "yn(0, 5)",
    "yn(1, 5)",
    "yn(-2, 5)",
    "yn(2, 0)",
    # argument-type errors (shared coercion)
    'sqrt("x")',
    "sqrt(true)",
    'pow(2, "x")',
    "ldexp(0.75, 1.5)",
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "math.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cases = [run(q, None, "json") for q in QUERIES]
    out_path.write_text(json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
