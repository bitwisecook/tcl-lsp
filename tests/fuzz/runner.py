"""Fuzz campaign runner.

Can be used standalone (``python -m tests.fuzz.runner``) or from pytest.
Saves failing cases to a corpus directory for regression testing.
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path

from .harness import FuzzResult, format_finding, run_differential
from .tcl_gen import GenConfig, TclGenerator

# Where to save failing scripts for regression
_CORPUS_DIR = Path(__file__).parent / "corpus"
_FINDINGS_DIR = Path(__file__).parent / "findings"


@dataclass
class CampaignStats:
    """Aggregate statistics for a fuzz campaign."""

    total: int = 0
    passed: int = 0
    mismatches: int = 0
    parse_errors: int = 0
    crashes: int = 0
    bad_inputs: int = 0
    elapsed_seconds: float = 0.0


def run_campaign(
    *,
    iterations: int = 1000,
    base_seed: int | None = None,
    config: GenConfig | None = None,
    use_tclsh: bool = True,
    tclsh_path: str | None = None,
    save_findings: bool = True,
    verbose: bool = False,
) -> tuple[CampaignStats, list[FuzzResult]]:
    """Run a fuzz campaign and return stats + any findings.

    Parameters
    ----------
    iterations:
        Number of random scripts to generate and test.
    base_seed:
        Starting seed; each iteration uses base_seed + i.
        If None, uses current time as base.
    config:
        Generator configuration.
    use_tclsh:
        Whether to compare against C tclsh.
    save_findings:
        Whether to save failing scripts to disk.
    verbose:
        Print progress to stderr.
    """
    if base_seed is None:
        base_seed = int(time.time())

    stats = CampaignStats()
    findings: list[FuzzResult] = []
    start = time.monotonic()

    for i in range(iterations):
        seed = base_seed + i
        gen = TclGenerator(seed=seed, config=config)
        script = gen.generate()

        # Heuristic: if the script has unbalanced delimiters it was
        # intentionally corrupted by the generator's bad_input_pct path.
        is_bad = (
            script.count("{") != script.count("}")
            or script.count('"') % 2 != 0
            or script.count("[") != script.count("]")
            or "\x00" in script
        )

        result = run_differential(
            script,
            seed=seed,
            use_tclsh=use_tclsh,
            tclsh_path=tclsh_path,
            bad_input=is_bad,
        )

        stats.total += 1

        if is_bad:
            stats.bad_inputs += 1

        if result.parse_error:
            stats.parse_errors += 1

        if any(m.kind == "crash" for m in result.mismatches):
            stats.crashes += 1

        if not result.ok:
            stats.mismatches += 1
            findings.append(result)
            if save_findings:
                _save_finding(result)
            if verbose:
                print(format_finding(result), file=sys.stderr)
        else:
            stats.passed += 1

        if verbose and (i + 1) % 100 == 0:
            elapsed = time.monotonic() - start
            rate = (i + 1) / elapsed
            print(
                f"  [{i + 1}/{iterations}] "
                f"{stats.passed} ok, {stats.mismatches} mismatch, "
                f"{stats.crashes} crash, {stats.bad_inputs} bad — {rate:.0f} scripts/sec",
                file=sys.stderr,
            )

    stats.elapsed_seconds = time.monotonic() - start
    return stats, findings


def _save_finding(result: FuzzResult) -> None:
    """Save a failing script and metadata to the findings directory."""
    _FINDINGS_DIR.mkdir(parents=True, exist_ok=True)
    stem = f"seed_{result.seed}"
    script_path = _FINDINGS_DIR / f"{stem}.tcl"
    script_path.write_text(result.script, encoding="utf-8")

    meta_path = _FINDINGS_DIR / f"{stem}.json"
    meta: dict[str, object] = {
        "seed": result.seed,
        "mismatches": [
            {
                "kind": m.kind,
                "description": m.description,
                "backend_a": m.backend_a,
                "backend_b": m.backend_b,
                "detail_a": m.detail_a[:500],
                "detail_b": m.detail_b[:500],
            }
            for m in result.mismatches
        ],
    }
    if result.parse_error:
        meta["parse_error"] = result.parse_error
    meta_path.write_text(json.dumps(meta, indent=2), encoding="utf-8")


def replay_finding(path: Path, *, use_tclsh: bool = True) -> FuzzResult:
    """Re-run a saved finding script through the harness."""
    script = path.read_text(encoding="utf-8")
    # Try to extract seed from filename
    seed = None
    if path.stem.startswith("seed_"):
        try:
            seed = int(path.stem.removeprefix("seed_"))
        except ValueError:
            pass
    return run_differential(script, seed=seed, use_tclsh=use_tclsh)


# CLI entry point


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(
        description="Tcl differential fuzzer",
    )
    parser.add_argument(
        "-n",
        "--iterations",
        type=int,
        default=1000,
        help="number of scripts to generate (default: 1000)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=None,
        help="base random seed",
    )
    parser.add_argument(
        "--no-tclsh",
        action="store_true",
        help="skip C tclsh comparison",
    )
    parser.add_argument(
        "--tclsh",
        type=str,
        default=None,
        help="explicit path to tclsh",
    )
    parser.add_argument(
        "--max-depth",
        type=int,
        default=4,
        help="max nesting depth (default: 4)",
    )
    parser.add_argument(
        "--bad-input-pct",
        type=float,
        default=0.05,
        help="fraction of inputs that are intentionally malformed (default: 0.05)",
    )
    parser.add_argument(
        "--coverage-guided",
        action="store_true",
        help="use coverage-guided mutation (AFL-style)",
    )
    parser.add_argument(
        "--replay",
        type=str,
        default=None,
        help="replay a saved finding .tcl file",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
    )
    args = parser.parse_args()

    if args.replay:
        result = replay_finding(
            Path(args.replay),
            use_tclsh=not args.no_tclsh,
        )
        if result.ok:
            print("PASS — no mismatches on replay")
        else:
            print(format_finding(result))
        sys.exit(0 if result.ok else 1)

    config = GenConfig(max_depth=args.max_depth, bad_input_pct=args.bad_input_pct)

    if args.coverage_guided:
        from .coverage_guide import run_coverage_guided

        cov_stats, findings = run_coverage_guided(
            iterations=args.iterations,
            base_seed=args.seed,
            config=config,
            verbose=args.verbose,
        )
        print(f"\n{'=' * 60}")
        print(f"Coverage-guided campaign: {cov_stats.total_executions} executions")
        print(f"  Corpus size:    {cov_stats.corpus_size}")
        print(f"  Total coverage: {cov_stats.total_coverage} edges")
        print(f"  New coverage:   {cov_stats.new_coverage_found} discoveries")
        print(f"  Mismatches:     {cov_stats.mismatches}")
        print(f"  Crashes:        {cov_stats.crashes}")
    else:
        stats, findings = run_campaign(
            iterations=args.iterations,
            base_seed=args.seed,
            config=config,
            use_tclsh=not args.no_tclsh,
            tclsh_path=args.tclsh,
            verbose=args.verbose,
        )
        print(f"\n{'=' * 60}")
        print(f"Fuzz campaign complete: {stats.total} scripts")
        print(f"  Passed:       {stats.passed}")
        print(f"  Mismatches:   {stats.mismatches}")
        print(f"  Parse errors: {stats.parse_errors}")
        print(f"  Crashes:      {stats.crashes}")
        print(f"  Bad inputs:   {stats.bad_inputs}")
        print(f"  Time:         {stats.elapsed_seconds:.1f}s")
        print(f"  Rate:         {stats.total / max(stats.elapsed_seconds, 0.001):.0f} scripts/sec")

    if findings:
        print(f"\nFindings saved to: {_FINDINGS_DIR}/")

    sys.exit(1 if findings else 0)


if __name__ == "__main__":
    main()
