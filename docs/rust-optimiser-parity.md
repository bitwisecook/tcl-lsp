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

## Implementation design (investigated 2026-06)

Mapped the Rust pipeline end to end. The pieces are in place but the
coupling is missing:

- `optimiser::manager::run_passes` runs **Propagation before Elimination**
  over a shared `PassContext`, so ordering is already correct.
- `PassContext` already carries `propagated_branch_uses` /
  `propagated_expr_stmts` / `propagated_use_groups` — but they are consumed
  **only inside `propagation.rs`** (for the O127 store-to-load path), not by
  `elimination.rs`.
- The **constant** path (`visit_simple_var_word` → O100) operates on a
  name→value `constants` map, **not** version-precise SSA, so it records
  nothing about *which* `(var, version)` uses it propagated.

A `set x <const>` becomes dead exactly when O100 propagates **every** use
of `x`. The safe rule (verified against Python's keep/remove split):

> Remove the def iff SCCP says `x` is `Const`, the def is pure
> (`assignment_safe_to_delete`), `x` has **≥ 1 use** and **all** uses are
> `UseKind::Operand` (reuse `build_adce_consumers`'s `keep_forever` to
> exclude Phi/Terminator), `x` is scalar, and `x` is not scope-aliased /
> cross-event / call-by-name / present in `collect_rmw_hidden_reads`. The
> `≥ 1 use` clause is what distinguishes `set x 42; puts $x` (remove —
> Python emits O109) from `set x 42; puts hello` and a proc-read global
> (keep — Python emits nothing): a zero-use top-level const is the
> "never-used / externally-consumed" case Python deliberately preserves.

### The blocking hazard — all-or-nothing grouping

The O109 removal of `set x 42` **must** be emitted in the same rewrite
**group** as the O100 rewrites of *every* use of `x`. O100 and O109 touch
disjoint spans, so `select_non_overlapping` keeps both in the simple case —
**but** in compound cases an O100 use-rewrite can be dropped by overlap
arbitration (it collides with an O112/O101 replacement). If the O109
removal survives while one O100 is dropped, the output is `puts $x` with
`x` no longer defined — a **miscompilation**, not a missed optimisation.

This is exactly what Python's `propagated_use_groups` provides. Landing the
fix therefore requires:

1. Making the O100 constant path record each propagated `(var, version)`
   use and its rewrite's group id (version-precise, not name-keyed).
2. Having elimination emit the O109 removal **into that group** so the
   removal applies only if *all* the use-rewrites do.
3. The safety guards above.
4. Re-running the optimiser corpus differential, the full Rust suite, the
   Python `test_optimiser*` equivalents, and the bytecode-compare gate.

## Status — coupling landed (post-selection survival check)

The dead-store coupling is implemented in
`optimiser::manager::couple_propagated_const_dead_stores`, using a design
that sidesteps the grouping problem entirely. Rust's group mechanism turned
out **not** to be application-gating (a dropped group member only clears the
survivors' group id; they still apply — see `helpers/select.rs`), so
grouping the removal with the propagations would not have prevented the
miscompile. Instead the removal is decided **after** `select_non_overlapping`,
conditional on the propagations having actually *survived*:

- a single `AssignConst` whose literal value has no substitution
  metacharacters (`$` `[` `]` `\`), SCCP-`Const`, scalar, not
  aliased/global/cross-event/RMW-hidden;
- every textual `$var` reference across the function consumed by a
  *surviving* propagation rewrite (`count_var_refs` == consumed count);
- the variable name appears as a bareword exactly once (the def target) —
  guards against by-name reads (`[set x]`, `info exists x`) the `$var` scan
  can't see;
- the removal span free of overlap with any selected rewrite.

Verified: `set x 42; puts $x` → `puts 42`; string-interpolation reads
removed; never-used / by-name-read / metacharacter-bearing defs kept;
**zero** new over-removals across the sample corpus (the 14 pre-existing
over-removals — e.g. `lset` not modelled as read-modify-write — are
untouched); behaviourally equivalent under `tclsh`.

### Widening + bug fixes (follow-up)

The by-name-read guard was relaxed to be quote/brace-aware
(`in_string_or_braces`): the variable name appearing as *literal text*
inside a `"…"` string or `{…}` braces (`puts "x=$x"`) is no longer mistaken
for a by-name read, so those defs now couple-remove and match Python
(`info exists x` and `[set x]`, which *are* by-name reads, still keep the
def). Two pre-existing correctness bugs were fixed in the same strip:

- **`lset` / `lpop` over-removal** — they read the list before rewriting it
  but lacked the `READS_BEFORE_WRITE` trait, so the generic lowering emitted
  `reads_own_defs: false` and a feeding `set lst {…}` was wrongly deleted
  (and flagged W220). Added the trait and wired the generic lowering's
  `reads_own_defs` to it.
- **Propagation into braced literals** — `puts {$x}` was rewritten to
  `puts 42}`. Both propagation paths now skip `Str` (braced) words before
  any fold.

Corpus exact 37 → 41; over-removals 14 → 11.

**O110 constant reassociation — done.** `reassociate_node`
(`optimiser::helpers::expr_simplify`) combines literal constants across
`+`/`-` and `*` chains (`$a + 1 + 2` → `$a + 3`, `$a * 2 * 3` → `$a * 6`),
keeping all non-constant terms and verified byte-for-byte against Python.

**Arithmetic identity/annihilator numeric guard — done.** The
operand-dropping identities (`x * 0`, `x + 0`, `x * 1`, `x / 1`, `x % 1`,
`x << 0`, `x ** 0,1`, bitwise `& | ^ 0`, unary `+x`) are now gated on the
dropped operand being provably numeric, threading a per-function numeric
name set (`numeric_var_names`, from `FunctionUnit.types`) through the
expr-simplify helpers (`NumericCtx`) into all three optimiser entry points.
Matches Python's `_is_provably_numeric_expr_node` gate; resolves the prior
`$a * 0 * 3` O110 divergence.

**Remaining follow-ups:**

- **`AssignExpr`/`AssignValue` SCCP-constant coupling** (`set x [expr
  {1+1}]`) — low corpus value; needs a surviving-fold check to tell the
  clean `[expr {…}]` fold from the quoted-expr `[expr "…"]` smell Python
  preserves.
- **O127 forwarding** and the 11 remaining pre-existing over-removals.

The diagnostic-side dead-store/unused parity (W210 / W211 / W220) is closed.

Diagnostic-side dead-store/unused parity (W210 / W211 / W220) is now
closed — see the analyser changes landed alongside this note.
