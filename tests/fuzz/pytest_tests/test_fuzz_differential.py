"""Differential fuzz tests — pytest integration.

Run with::

    uv run pytest tests/test_fuzz_differential.py -v
    uv run pytest tests/test_fuzz_differential.py -v -k campaign  # full campaign
    uv run pytest tests/test_fuzz_differential.py -v -k regression  # saved findings only

The ``FUZZ_ITERATIONS`` environment variable controls campaign size
(default: 200 in CI, override with e.g. ``FUZZ_ITERATIONS=5000``).

The ``FUZZ_SEED`` environment variable pins the base seed for
reproducibility.
"""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from tests.fuzz.harness import format_finding, run_differential
from tests.fuzz.runner import run_campaign
from tests.fuzz.tcl_gen import GenConfig, TclGenerator

pytestmark = pytest.mark.slow

# Configuration

_DEFAULT_ITERATIONS = 200
_FINDINGS_DIR = Path(__file__).parent / "fuzz" / "findings"
_CORPUS_DIR = Path(__file__).parent / "fuzz" / "corpus"


def _iterations() -> int:
    return int(os.environ.get("FUZZ_ITERATIONS", _DEFAULT_ITERATIONS))


def _base_seed() -> int | None:
    val = os.environ.get("FUZZ_SEED")
    return int(val) if val else None


# Smoke test: generator produces valid-looking Tcl


class TestGenerator:
    """Basic sanity checks on the Tcl generator."""

    def test_generates_non_empty(self) -> None:
        gen = TclGenerator(seed=42)
        script = gen.generate()
        assert len(script) > 10
        assert "\n" in script

    def test_balanced_braces(self) -> None:
        gen = TclGenerator(seed=123)
        script = gen.generate()
        assert script.count("{") == script.count("}")

    def test_balanced_quotes(self) -> None:
        gen = TclGenerator(seed=456)
        script = gen.generate()
        assert script.count('"') % 2 == 0

    def test_deterministic(self) -> None:
        a = TclGenerator(seed=99).generate()
        b = TclGenerator(seed=99).generate()
        assert a == b

    def test_different_seeds_different_output(self) -> None:
        a = TclGenerator(seed=1).generate()
        b = TclGenerator(seed=2).generate()
        assert a != b

    def test_bad_input_pct_produces_some_bad(self) -> None:
        """With bad_input_pct=1.0, every script should be corrupted."""
        cfg = GenConfig(bad_input_pct=1.0)
        bad_count = 0
        for seed in range(50):
            gen = TclGenerator(seed=seed, config=cfg)
            script = gen.generate()
            is_bad = (
                script.count("{") != script.count("}")
                or script.count('"') % 2 != 0
                or script.count("[") != script.count("]")
                or "\x00" in script
            )
            if is_bad:
                bad_count += 1
        # All should be corrupted (some corruptions like backslash-eol
        # may not trip the heuristic, so allow a generous margin)
        assert bad_count >= 35, f"expected mostly bad, got {bad_count}/50"

    def test_bad_input_pct_zero_no_corruption(self) -> None:
        """With bad_input_pct=0, no null bytes or lone quotes appear."""
        cfg = GenConfig(bad_input_pct=0.0)
        for seed in range(50):
            gen = TclGenerator(seed=seed, config=cfg)
            script = gen.generate()
            assert "\x00" not in script, f"seed={seed}: null byte found"
            assert script.count('"') % 2 == 0, f"seed={seed}: odd quotes"


# VM optimised vs unoptimised


class TestOptimiserEquivalence:
    """Verify optimised and unoptimised VM produce identical results."""

    @pytest.mark.parametrize("seed", range(50))
    def test_optimiser_equivalence(self, seed: int) -> None:
        gen = TclGenerator(seed=seed, config=GenConfig(max_depth=3))
        script = gen.generate()
        result = run_differential(script, seed=seed, use_tclsh=False)

        # Filter to only vm vs vm_opt mismatches
        opt_mismatches = [
            m for m in result.mismatches if {m.backend_a, m.backend_b} == {"vm", "vm_opt"}
        ]

        if opt_mismatches:
            pytest.fail(f"Optimiser mismatch at seed={seed}:\n" + format_finding(result))


# Full differential campaign


class TestFuzzCampaign:
    """Run a differential fuzz campaign."""

    def test_campaign(self) -> None:
        iterations = _iterations()
        stats, findings = run_campaign(
            iterations=iterations,
            base_seed=_base_seed(),
            use_tclsh=True,
            verbose=False,
        )

        # Report stats
        print(
            f"\nFuzz: {stats.total} scripts, "
            f"{stats.passed} ok, {stats.mismatches} mismatch, "
            f"{stats.crashes} crash, {stats.bad_inputs} bad, "
            f"{stats.elapsed_seconds:.1f}s"
        )

        if findings:
            # Show first 3 findings in detail
            details = "\n\n".join(format_finding(f) for f in findings[:3])
            pytest.fail(f"{len(findings)} mismatches in {iterations} scripts:\n\n" + details)


# Regression tests from saved findings


def _find_saved_findings() -> list[Path]:
    """Collect .tcl files from the findings directory."""
    if not _FINDINGS_DIR.is_dir():
        return []
    return sorted(_FINDINGS_DIR.glob("*.tcl"))


class TestRegressions:
    """Re-run any saved findings to check for regressions."""

    @pytest.mark.parametrize(
        "finding_path",
        _find_saved_findings(),
        ids=[p.stem for p in _find_saved_findings()],
    )
    def test_regression(self, finding_path: Path) -> None:
        script = finding_path.read_text(encoding="utf-8")
        result = run_differential(script, use_tclsh=True)
        if not result.ok:
            pytest.fail(f"Regression in {finding_path.name}:\n" + format_finding(result))


# Corpus replay


def _find_corpus_scripts() -> list[Path]:
    """Collect .tcl files from the seed corpus."""
    if not _CORPUS_DIR.is_dir():
        return []
    return sorted(_CORPUS_DIR.glob("*.tcl"))


class TestCorpus:
    """Run seed corpus scripts through the differential harness."""

    @pytest.mark.parametrize(
        "corpus_path",
        _find_corpus_scripts(),
        ids=[p.stem for p in _find_corpus_scripts()],
    )
    def test_corpus_script(self, corpus_path: Path) -> None:
        script = corpus_path.read_text(encoding="utf-8")
        result = run_differential(script, use_tclsh=True)
        if not result.ok:
            pytest.fail(f"Corpus mismatch in {corpus_path.name}:\n" + format_finding(result))
