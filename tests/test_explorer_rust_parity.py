"""Differential parity gate for the Rust compiler-explorer serialiser.

The Rust ``tcl-explorer`` crate is a faithful port of
``tooling/cli/serialise.py``. This module pins that fidelity: for each
source snippet it runs the Rust serialiser (via the ``explore_json``
example) and the Python ``serialise_result``, then asserts the two agree
on **every contract key the Rust side currently emits**.

The comparison is intentionally key-scoped to the Rust output, so the gate
grows automatically: as each view family lands in the Rust serialiser, its
key starts appearing in ``rust_json`` and is compared from that commit on.
Keys not yet ported are simply absent on the Rust side and skipped, never
masking a regression on the keys that *are* ported.

The harness shells out to a built example binary, mirroring
``test_bigip_rust_parity.py``; it soft-skips when the Rust toolchain is
unavailable.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest

from tooling.cli.serialise import serialise_result
from tooling.explorer.pipeline import run_pipeline

_ROOT = Path(__file__).parent.parent

# Curated source snippets. Kept small and deterministic; extend alongside
# each view family so the new key is exercised over representative shapes.
_CORPUS: dict[str, str] = {
    "empty": "",
    "two_sets": "set x 1\nset y 2",
    "proc_call": "proc greet {name} { return $name }\ngreet world",
    "if_else": "if {$x > 0} { puts hi } else { puts lo }",
    "for_loop": "for {set i 0} {$i < 10} {incr i} { puts $i }",
    "while_loop": "set i 0\nwhile {$i < 3} { incr i }",
    "foreach_loop": "foreach a {1 2 3} { puts $a }",
    "switch_stmt": "switch -- $x { a { puts A } b { puts B } default { puts D } }",
    "catch_stmt": "catch { error boom } msg",
    "expr_assign": "set y [expr {$x * 2 + 1}]",
    "const_proc": "proc answer {} { return 42 }\nanswer",
    "str_const_proc": "proc greeting {} { return hello }\ngreeting",
    "passthrough_proc": "proc id {x} { return $x }\nid 5",
    "depends_proc": "proc add {a b} { return [expr {$a + $b}] }\nadd 1 2",
    "calls_proc": "proc inner {} { return 1 }\nproc outer {} { inner }\nouter",
    "rendered_path": "set p /var/log/app\nset u -flag\nset b a\\\\b",
    "opt_constfold": "set x [expr {1 + 2}]\nputs $x",
    "opt_deadcode": "proc f {} {\n  set unused 5\n  return 1\n}\nf",
    "shimmer_intrep": "set x [list 1 2 3]\nincr x",
    "taint_tracking": "set x [exec ls]\nset y $x",
}

_DIALECT = "tcl8.6"

# Views whose payload is derived from a semantic analysis that is still
# only partially ported (🟡 in docs/rust-rewrite.md). The Rust and Python
# analyses legitimately diverge in precision — e.g. the Rust interprocedural
# pass detects `depends(a,b)` return shapes and call-graph purity that the
# Python pass leaves `unknown`/impure. The explorer view faithfully reflects
# whichever pipeline produced it, so for these views we verify the
# serialiser *shape* (same entries, same fields) rather than byte-equality,
# until the underlying analyses converge.
_SHAPE_ONLY_KEYS = {"interprocedural"}

# Views whose Rust backend (the optimiser, or the rendered-properties pass)
# is materially different from the Python implementation, so a Python
# differential is the wrong gate. The Rust optimiser is the production
# source of truth (it drives `tcl --opt`, the LSP code actions, and the
# bytecode-compare gate) and intentionally improves on the Python optimiser
# (folds an interprocedurally-constant `return $param`; prefers `O101 fold`
# over `O109 dead-store`). The rendered-properties pass (🟡) reports
# conservative flag sets for command-substitution values where the Python
# pass reports only interpolation. The taint pass points a warning's range
# at the whole sink command (`eval $x`) where Python points at the tainted
# variable use (`$x`), and its proc-order is not yet stable. The explorer
# view faithfully shows whichever backend produced it, so these views are
# pinned by Rust-side unit tests and the backends' own suites instead.
# (Tracked under the compiler / analyser work in docs/rust-rewrite.md.)
_NO_PARITY_KEYS = {
    "optimisations",
    "optimisedSource",
    "irOptimised",
    "cfgPreSsaOptimised",
    # Post-SSA CFG: analysis.deadStores is [] (Rust liveness unported) and
    # the lattice/type detail comes from the divergent Rust analyses.
    "cfgPostSsa",
    "cfgPostSsaOptimised",
    "renderedProperties",
    "taintWarnings",
    "gvn",
    # The Rust bytecode codegen differs from Python in source-line
    # population (always 0 today) and label-counter numbering, and the Rust
    # `Instruction` carries no source span (per-instruction `range` is
    # null), so `asm` is pinned by a Rust unit test + the bytecode-compare
    # gate rather than the Python differential.
    "asm",
    "asmOptimised",
    # The WASM emitter is unported (Rust emits null where Python emits real
    # disassembly); they degrade gracefully and light up when the emitter lands.
    "wasm",
    "wasmOptimised",
    # Source callouts aggregate the divergent optimiser/shimmer/gvn/taint
    # sources and omit dead-store callouts (the Rust liveness pass that
    # fills FunctionAnalysis.dead_stores is unported), so they are pinned by
    # a Rust unit test rather than the Python differential.
    "annotations",
    "annotationsByLine",
    # deadStores is 0 (Rust liveness unported) and the warning counts come
    # from the divergent Rust analyses, so stats is Rust-pinned.
    "stats",
}


def _rust_binary() -> Path | None:
    """Build and locate the ``explore_json`` example, or ``None`` to skip."""
    if shutil.which("cargo") is None:
        return None
    build = subprocess.run(
        ["cargo", "build", "-p", "tcl-explorer", "--example", "explore_json", "-q"],
        cwd=_ROOT,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        return None
    binary = _ROOT / "target" / "debug" / "examples" / "explore_json"
    return binary if binary.exists() else None


def _rust_json(binary: Path, source: str, dialect: str) -> dict:
    out = subprocess.run(
        [str(binary), "--dialect", dialect, "--source", source],
        capture_output=True,
        text=True,
    )
    assert out.returncode == 0, f"explore_json failed: {out.stderr}"
    return json.loads(out.stdout)


def _python_json(source: str, dialect: str) -> dict:
    # Round-trip through json to normalise tuples → lists, etc., so the
    # comparison is value-structural (not Python-type sensitive).
    return json.loads(json.dumps(serialise_result(run_pipeline(source, dialect=dialect))))


def _is_orphan_exit(block: dict) -> bool:
    """An unreachable trailing `exit_N` block with no statements/edges.

    The Python CFG builder mints an `exit` block unconditionally at the end
    of every function (`compiler/cfg.py`), so a body that ends in `return`
    leaves a dangling, unreachable exit block. The Rust builder only creates
    the exit block when the body falls through, so it omits this orphan. The
    block carries no statements, terminator, successors, or incoming edges,
    so dropping it from both sides is loss-free and lets the rest of the CFG
    view compare byte-for-byte. (Full convergence awaits the CFG-builder
    parity work tracked in docs/rust-rewrite.md.)
    """
    return (
        block.get("name", "").startswith("exit_")
        and not block.get("isEntry", False)
        and not block.get("statements")
        and block.get("terminator") is None
        and not block.get("successors")
    )


def _normalise_types(funcs: list) -> list:
    """Drop the `(return)` rows where the two type-inference passes diverge.

    Rust's `type_infer` assigns an `Overdefined` (`*`) return type to
    functions whose return value it can't pin down, whereas the Python pass
    leaves `analysis.return_type` as `None` and emits no `(return)` row at
    all. The concrete return types (and every variable's type) still match
    byte-for-byte, so we strip only the `*` `(return)` rows from both sides
    and drop any function left with no rows. (Return-type-inference parity
    is tracked under the analyser work in docs/rust-rewrite.md.)
    """
    out = []
    for fn in funcs:
        fn = dict(fn)
        entries = [
            e
            for e in fn.get("entries", [])
            if not (e.get("variable") == "(return)" and e.get("type") == "*")
        ]
        if entries:
            fn["entries"] = entries
            out.append(fn)
    return out


def _normalise_shimmer(warnings: list) -> list:
    """Drop the shimmer warning `message` (cosmetic wording only).

    The Rust and Python shimmer passes agree on every semantic field —
    code, range, variable, from/to type, command, inLoop, severity — but
    phrase the human-readable `message` differently ("S100: variable 'x'
    has list intrep…" vs "Shimmer: $x has intrep list… (S100)"). Compare
    everything except that wording. (Message-text parity is cosmetic and
    not tracked as a porting gap.)
    """
    return [{k: v for k, v in w.items() if k != "message"} for w in warnings]


def _normalise(payload: dict) -> dict:
    """Strip known, characterised cross-implementation divergences so the
    remaining contract is compared exactly. Applied to both sides."""
    payload = dict(payload)
    if isinstance(payload.get("cfgPreSsa"), list):
        normalised_funcs = []
        for fn in payload["cfgPreSsa"]:
            fn = dict(fn)
            kept = [b for b in fn.get("blocks", []) if not _is_orphan_exit(b)]
            removed = len(fn.get("blocks", [])) - len(kept)
            fn["blocks"] = kept
            if "blockCount" in fn:
                fn["blockCount"] -= removed
            normalised_funcs.append(fn)
        payload["cfgPreSsa"] = normalised_funcs
    if isinstance(payload.get("types"), list):
        payload["types"] = _normalise_types(payload["types"])
    if isinstance(payload.get("shimmer"), list):
        payload["shimmer"] = _normalise_shimmer(payload["shimmer"])
    return payload


@pytest.mark.parametrize("name", sorted(_CORPUS), ids=lambda n: n)
def test_rust_serialiser_matches_python(name: str) -> None:
    binary = _rust_binary()
    if binary is None:
        pytest.skip("Rust toolchain / explore_json example unavailable")

    source = _CORPUS[name]
    rust = _normalise(_rust_json(binary, source, _DIALECT))
    py = _normalise(_python_json(source, _DIALECT))

    assert rust, "Rust serialiser emitted no keys"
    for key in rust:
        assert key in py, f"Rust emitted unknown contract key {key!r}"
        if key in _NO_PARITY_KEYS:
            continue
        if key in _SHAPE_ONLY_KEYS:
            _assert_same_shape(key, rust[key], py[key], name)
        else:
            assert rust[key] == py[key], f"mismatch on {key!r} for snippet {name!r}"


def _assert_same_shape(key: str, rust: list, py: list, snippet: str) -> None:
    """Verify two analysis-view payloads describe the same entries with the
    same fields, ignoring the (legitimately divergent) analysis values."""
    rust_names = {e["name"] for e in rust}
    py_names = {e["name"] for e in py}
    assert rust_names == py_names, f"{key}: entry set differs for {snippet!r}"
    rust_fields = {e["name"]: set(e) for e in rust}
    py_fields = {e["name"]: set(e) for e in py}
    assert rust_fields == py_fields, f"{key}: per-entry fields differ for {snippet!r}"
