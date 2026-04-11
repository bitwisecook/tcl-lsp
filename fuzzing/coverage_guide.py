"""Coverage-guided fuzzing for the Tcl VM.

Uses Python's ``sys.settrace`` to collect branch/line coverage of the VM
and parser during each script execution.  Scripts that discover new code
paths are saved to the corpus and used as seeds for further mutation.

This mimics AFL's core loop:
  1. Pick a seed from the corpus.
  2. Mutate it (structurally or textually).
  3. Execute and measure coverage.
  4. If new coverage is found, add the mutant to the corpus.
  5. Compare outputs across backends (differential oracle).

The coverage map is a set of (filename, lineno) tuples restricted to
the ``vm/`` and ``core/`` packages — we only care about exercising *our*
code, not stdlib.
"""

from __future__ import annotations

import io
import random
import re
import sys
import threading
import types
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .harness import FuzzResult, RunResult, run_differential
from .tcl_gen import GenConfig, TclGenerator, corrupt_script

_TraceFunc = Callable[[types.FrameType, str, Any], "_TraceFunc"]

# Coverage collection

# Packages whose source lines count toward the coverage map.
_INTERESTING_PREFIXES: tuple[str, ...] = ()


def _init_prefixes() -> tuple[str, ...]:
    """Lazily compute the absolute path prefixes for vm/ and core/."""
    global _INTERESTING_PREFIXES
    if _INTERESTING_PREFIXES:
        return _INTERESTING_PREFIXES
    repo = Path(__file__).resolve().parent.parent.parent
    _INTERESTING_PREFIXES = (
        str(repo / "vm"),
        str(repo / "core"),
    )
    return _INTERESTING_PREFIXES


def _make_tracer(coverage_set: set[tuple[str, int]]) -> _TraceFunc:
    """Create a trace function that records (file, line) hits."""
    prefixes = _init_prefixes()

    def tracer(frame: types.FrameType, event: str, arg: object) -> _TraceFunc:
        if event == "line" or event == "call":
            filename = frame.f_code.co_filename
            for prefix in prefixes:
                if filename.startswith(prefix):
                    coverage_set.add((filename, frame.f_lineno))
                    break
        return tracer

    return tracer


def collect_coverage(
    script: str, *, optimise: bool = False, timeout: float = 5.0
) -> tuple[RunResult, set[tuple[str, int]]]:
    """Run a script through the VM and return (result, coverage_set).

    Uses a daemon thread with *timeout* seconds to prevent infinite loops
    from hanging the process.
    """
    from vm.interp import TclInterp
    from vm.types import TclError, TclReturn

    coverage: set[tuple[str, int]] = set()
    result_box: list[RunResult] = []
    stdout_buf = io.StringIO()
    stderr_buf = io.StringIO()

    def _target() -> None:
        old_trace = sys.gettrace()
        try:
            sys.settrace(_make_tracer(coverage))
            interp = TclInterp(optimise=optimise, source_init=False)
            interp.channels["stdout"] = stdout_buf
            interp.channels["stderr"] = stderr_buf
            interp.eval(script)
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=0,
                )
            )
        except TclError as e:
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=1,
                    error_message=str(e.message) if hasattr(e, "message") else str(e),
                )
            )
        except TclReturn:
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=0,
                )
            )
        except (SystemExit, KeyboardInterrupt):
            raise
        except Exception as e:
            result_box.append(
                RunResult(
                    stdout=stdout_buf.getvalue(),
                    stderr=stderr_buf.getvalue(),
                    return_code=2,
                    error_message=f"CRASH: {type(e).__name__}: {e}",
                )
            )
        finally:
            sys.settrace(old_trace)

    t = threading.Thread(target=_target, daemon=True)
    t.start()
    t.join(timeout)

    if t.is_alive():
        return RunResult(
            stdout="",
            stderr="",
            return_code=2,
            error_message="TIMEOUT",
        ), coverage

    if result_box:
        return result_box[0], coverage

    return RunResult(
        stdout="", stderr="", return_code=2, error_message="CRASH: no result"
    ), coverage


# Mutation strategies


def mutate_script(script: str, rng: random.Random, *, bad_input_pct: float = 0.05) -> str:
    """Apply a random structural mutation to a Tcl script.

    Most mutations preserve syntactic validity (balanced delimiters).
    With probability *bad_input_pct*, a corruption is applied instead so
    that error-recovery paths get exercised.
    """
    if bad_input_pct > 0 and rng.random() < bad_input_pct:
        return corrupt_script(script, rng)

    strategies = [
        _mutate_number,
        _mutate_string_literal,
        _mutate_operator,
        _mutate_duplicate_line,
        _mutate_remove_line,
        _mutate_swap_lines,
        _mutate_inject_statement,
    ]
    strategy = rng.choice(strategies)
    result = strategy(script, rng)
    # Verify braces/quotes are still balanced
    if result.count("{") != result.count("}"):
        return script  # reject mutation
    if result.count('"') % 2 != 0:
        return script  # reject mutation
    return result


def _mutate_number(script: str, rng: random.Random) -> str:
    """Replace a random integer literal with a different value."""
    # Find all integer literals (standalone numbers)
    matches = list(re.finditer(r"(?<![a-zA-Z_.])\b(\d+)\b(?![a-zA-Z_.])", script))
    if not matches:
        return script
    match = rng.choice(matches)
    old_val = int(match.group(1))
    mutations = [0, 1, -1, old_val + 1, old_val - 1, old_val * 2, 0x7FFFFFFF, -old_val]
    new_val = rng.choice(mutations)
    return script[: match.start(1)] + str(new_val) + script[match.end(1) :]


def _mutate_string_literal(script: str, rng: random.Random) -> str:
    """Replace a quoted string with a different one."""
    matches = list(re.finditer(r'"([^"]*)"', script))
    if not matches:
        return script
    match = rng.choice(matches)
    length = rng.randint(0, 15)
    safe = "abcdefghijklmnopqrstuvwxyz0123456789 .,;:-_"
    new_str = "".join(rng.choice(safe) for _ in range(length))
    return script[: match.start(1)] + new_str + script[match.end(1) :]


def _mutate_operator(script: str, rng: random.Random) -> str:
    """Swap an arithmetic/comparison operator."""
    ops = {
        "+": "-",
        "-": "+",
        "*": "/",
        "/": "*",
        "<": ">",
        ">": "<",
        "==": "!=",
        "!=": "==",
        "<=": ">=",
        ">=": "<=",
    }
    # Try to find an operator in the script
    for old, new in rng.sample(list(ops.items()), len(ops)):
        idx = script.find(f" {old} ")
        if idx >= 0:
            return script[:idx] + f" {new} " + script[idx + len(old) + 2 :]
    return script


def _mutate_duplicate_line(script: str, rng: random.Random) -> str:
    """Duplicate a random line."""
    lines = script.split("\n")
    if len(lines) < 2:
        return script
    idx = rng.randint(0, len(lines) - 1)
    lines.insert(idx + 1, lines[idx])
    return "\n".join(lines)


def _mutate_remove_line(script: str, rng: random.Random) -> str:
    """Remove a random non-brace line."""
    lines = script.split("\n")
    if len(lines) < 3:
        return script
    # Don't remove lines that contain only braces
    candidates = [i for i, ln in enumerate(lines) if ln.strip() not in ("{", "}", "")]
    if not candidates:
        return script
    idx = rng.choice(candidates)
    del lines[idx]
    return "\n".join(lines)


def _mutate_swap_lines(script: str, rng: random.Random) -> str:
    """Swap two adjacent lines."""
    lines = script.split("\n")
    if len(lines) < 3:
        return script
    idx = rng.randint(0, len(lines) - 2)
    lines[idx], lines[idx + 1] = lines[idx + 1], lines[idx]
    return "\n".join(lines)


_INJECT_STMTS = [
    "set _fuzz_tmp [expr {1 + 1}]",
    'catch {error "fuzz"}',
    'string length "test"',
    "llength {a b c}",
    "expr {0 == 0}",
    'append _fuzz_buf "x"',
    "lappend _fuzz_list item",
]


def _mutate_inject_statement(script: str, rng: random.Random) -> str:
    """Inject a random safe statement at a random position."""
    lines = script.split("\n")
    stmt = rng.choice(_INJECT_STMTS)
    pos = rng.randint(0, len(lines))
    lines.insert(pos, stmt)
    return "\n".join(lines)


# Coverage-guided campaign


@dataclass
class CoverageStats:
    """Statistics from a coverage-guided campaign."""

    total_executions: int = 0
    corpus_size: int = 0
    total_coverage: int = 0
    new_coverage_found: int = 0
    mismatches: int = 0
    crashes: int = 0


def run_coverage_guided(
    *,
    iterations: int = 1000,
    base_seed: int | None = None,
    initial_corpus: list[str] | None = None,
    config: GenConfig | None = None,
    verbose: bool = False,
) -> tuple[CoverageStats, list[FuzzResult]]:
    """Run a coverage-guided differential fuzz campaign.

    Parameters
    ----------
    iterations:
        Total number of executions.
    base_seed:
        Random seed for reproducibility.
    initial_corpus:
        Starting set of Tcl scripts. If None, generates random seeds.
    config:
        Generator config for initial random seeds.
    verbose:
        Print progress.
    """
    import time

    rng = random.Random(base_seed)
    stats = CoverageStats()
    findings: list[FuzzResult] = []

    # Global coverage bitmap — union of all coverage seen so far
    global_coverage: set[tuple[str, int]] = set()

    # Corpus of interesting scripts
    corpus: list[str] = list(initial_corpus or [])

    # Generate initial corpus if empty
    if not corpus:
        for i in range(min(20, iterations)):
            gen = TclGenerator(seed=rng.randint(0, 2**31), config=config)
            corpus.append(gen.generate())

    start = time.monotonic()

    bad_pct = config.bad_input_pct if config else GenConfig().bad_input_pct

    for i in range(iterations):
        # Pick a seed from corpus and mutate
        parent = rng.choice(corpus)
        script = mutate_script(parent, rng, bad_input_pct=bad_pct)

        # Run with coverage
        run_result, coverage = collect_coverage(script)
        stats.total_executions += 1

        # Check for new coverage
        new_edges = coverage - global_coverage
        if new_edges:
            stats.new_coverage_found += 1
            global_coverage |= new_edges
            corpus.append(script)

            if verbose:
                print(
                    f"  [{i}/{iterations}] NEW COVERAGE: "
                    f"+{len(new_edges)} edges (total: {len(global_coverage)}, "
                    f"corpus: {len(corpus)})",
                    file=sys.stderr,
                )

        # Detect intentionally corrupted scripts
        is_bad = (
            script.count("{") != script.count("}")
            or script.count('"') % 2 != 0
            or script.count("[") != script.count("]")
            or "\x00" in script
        )

        # Differential comparison (optimised vs unoptimised)
        result = run_differential(script, seed=i, use_tclsh=False, bad_input=is_bad)
        if not result.ok:
            stats.mismatches += 1
            findings.append(result)
            if any(m.kind == "crash" for m in result.mismatches):
                stats.crashes += 1
            if verbose:
                print(f"  [{i}/{iterations}] MISMATCH:", file=sys.stderr)
                for m in result.mismatches:
                    print(f"    {m.kind}: {m.description}", file=sys.stderr)

        if verbose and (i + 1) % 100 == 0:
            elapsed = time.monotonic() - start
            rate = (i + 1) / elapsed
            print(
                f"  [{i + 1}/{iterations}] "
                f"coverage={len(global_coverage)}, corpus={len(corpus)}, "
                f"mismatches={stats.mismatches} — {rate:.0f}/sec",
                file=sys.stderr,
            )

    stats.corpus_size = len(corpus)
    stats.total_coverage = len(global_coverage)
    return stats, findings
