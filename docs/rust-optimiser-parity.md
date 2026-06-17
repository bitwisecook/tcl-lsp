# Rust optimiser parity snapshot

This note records the measured gap between the Python optimiser
(`compiler.optimiser`) and the native Rust optimiser
(`tcl-compiler::optimiser`, driven by `tcl opt`). The historical
`TCL_LSP_RUST_OPTIMISER` opt-in flag is gone — the Rust optimiser is the
production path now, so parity is measured by a same-entry differential
(`tcl opt` vs `optimise_source_multipass`).

## How to reproduce

```bash
# Final-source differential over the sample corpus.
python - <<'PY'
import subprocess, glob, re
from pathlib import Path
from compiler.registry.runtime import configure_signatures
configure_signatures()
from compiler.optimiser import optimise_source_multipass
for f in sorted(glob.glob("samples/**/*.tcl", recursive=True)):
    py, _, _ = optimise_source_multipass(Path(f).read_text())
    out = subprocess.run(["target/debug/tcl", "opt", f],
                         capture_output=True, text=True).stdout
    rust = re.split(r'\n+# -+\n', out)[0]
    if rust.rstrip("\n") != py.rstrip("\n"):
        print("DIVERGE", f)
PY
```

## Current snapshot (2026-06)

Final-source parity over the 79-file sample corpus: **37 exact / 42
divergent**. Both pipelines already iterate to a fixpoint
(`optimise_source_multipass` / the loop in `tcl-cli::run_opt`), so the
remaining gap is *per-pass content*, not the loop.

Divergence classes:

| Class | Count | Meaning |
|---|---:|---|
| Rust keeps more lines | 28 | Rust leaves a dead store Python removes |
| Rust removes more lines | 5 | Rust eliminates something Python keeps |
| Reorder / same line-count | 9 | O110 reassociation, expr canonicalisation, wording |

## Root cause of the dominant class (28 "Rust keeps more")

The canonical reproducer:

```tcl
set x 42
puts $x
```

- **Python** → `puts 42`. Its constant-propagation analysis emits the
  propagation (O100/O102) **and**, in the same pass, the O109 dead-store
  removal of `set x 42` — because propagating the value eliminates the
  variable's *last* use, so the store becomes dead. Propagation and the
  consequent DCE are **coupled** in one `find_optimisations` pass.
- **Rust** → `set x 42` + `puts 42`. Propagation (O102) and dead-store
  elimination are *separate* passes. The multipass's second iteration sees
  `set x 42` with `x` now unread, but `optimiser::elimination` classifies a
  set-once dead def as **O126** (`emit_dead_stores_and_unused`, the
  `any_other_live` split), and **O126 is suppressed at the top level**
  (`elimination.rs` "Top-level never emits O126"). So the store is never
  removed.

The subtlety that makes this hard to match: Python is *also* conservative
about top-level never-used stores —

```tcl
set x 42
puts hello      ;# x never read at all  → Python keeps `set x 42`
```

Python emits **no** removal here. So the distinguishing factor is whether
the store *had* a use that propagation eliminated (→ remove, O109) versus a
store that was *never* read (→ keep at top level, O126). Rust's per-pass
`any_other_live` classification cannot tell these apart across multipass
iterations, because by iteration 2 both look identical (a set-once dead
def).

### Fix options (for a dedicated follow-up)

1. **Couple propagation + DCE** (matches Python): when the constant-ref
   propagation pass propagates a variable's last remaining use, also emit
   the O109 removal of its defining `set` in the same pass. Highest
   fidelity; localised to the propagation pass.
2. **Carry "had-a-use" state across multipass iterations**: remember which
   variables lost a use to propagation so iteration 2 can emit O109 (not
   O126) and remove them at the top level. More invasive.

Either way the top-level / global-safety guards must be preserved — a
top-level `set g 1` read by a proc via `global g` must **not** be removed
(verified: Python keeps it). The `globals_written_by_procs` set the
analyser already computes for the W210 top-level suppression is the right
input.

## The other classes

- **9 reorder / same-line**: O110 constant reassociation
  (`$a + 1 + 2` → `$a + 3`), expr canonicalisation, and DeMorgan-style
  rewrites Rust does not yet perform. Independent of the DCE gap.
- **5 Rust removes more**: cases where Rust over-eliminates relative to
  Python — investigate individually; some may be Rust correctly improving
  on Python (it is the production optimiser and intentionally diverges on a
  few classes — see `tests/test_explorer_rust_parity.py::_NO_PARITY_KEYS`),
  others may be a missing guard.
- **O127 forwarding** (doc GAP-B1) remains unported.

## Status

The dead-store-coupling fix (option 1) is the single highest-impact change
— it closes the 28-file dominant class. It is correctness-sensitive
(removing a live store produces wrong code), so it warrants its own
strip with the global-safety guards and a corpus + bytecode-compare gate
re-run, not a drive-by edit.

Diagnostic-side dead-store/unused parity (W210 / W211 / W220) is now
closed — see the analyser changes landed alongside this note.
