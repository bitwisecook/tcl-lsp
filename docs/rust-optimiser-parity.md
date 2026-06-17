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
    # IMPORTANT: compare against `--profile aggressive` — that is the only
    # multipass-to-fixpoint Rust profile, matching Python's
    # `optimise_source_multipass`. The default `full` profile is SINGLE-PASS
    # (see `tcl-cli::run_opt`), so comparing it against the multipass Python
    # library understates parity (a fold that needs propagate-then-fold across
    # two passes never lands in one pass).
    out = subprocess.run(["target/debug/tcl", "opt", "--profile", "aggressive", f],
                         capture_output=True, text=True).stdout
    # Footer marker is `\n# ----…\n# optimised: N rewrite(s)`; the files' own
    # `# ----` comment lines must NOT be mistaken for it.
    m = re.search(r'\n# -+\n# optimised: \d+ rewrite', out)
    rust = out[:m.start()] if m else out
    if rust.rstrip("\n") != py.rstrip("\n"):
        print("DIVERGE", f)
PY
```

## Current snapshot (2026-06)

Final-source parity over the 79-file sample corpus, comparing
`tcl opt --profile aggressive` (multipass) against
`optimise_source_multipass`: **44 exact / 35 divergent** (was 37/42 at the
start of the coupling work). The remaining gap is *per-pass content*, not
the loop.

> **Measurement caveat (corrected 2026-06):** an earlier version of the
> reproduce script compared the *default* `tcl opt` profile (`full`, which
> is **single-pass** — only `aggressive` iterates to a fixpoint) against the
> multipass Python library, and used a `# -+` footer split that also matched
> the corpus files' own `# ----` comment separators. Both are fixed above.
> Under the single-pass `full` profile the count is 43/79; the genuine
> per-pass gaps are the 35 that remain even under multipass.

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

**`AssignExpr`/`AssignValue` SCCP-constant coupling — done.** A computed
`set x [expr {1+1}]` whose value SCCP proves `Const` now couple-removes
exactly like a literal `set x 42` once its uses are propagated. The
`couple_const_dead_stores_in_function` def gate was extended to accept
`AssignExpr` / `AssignValue` (rendering the SCCP constant via
`format_constant` for the substitution-metacharacter guard), and the
removal-application loop now *supersedes* any selected rewrite fully
contained in the deleted def line (the O101 expr-fold inside
`set x [expr {…}]`) rather than treating it as a blocking overlap; a
*partial* overlap still skips the removal. The investigated "quoted-expr
smell" turned out moot — Python folds and removes `[expr "1+1"]`
identically to `[expr {1+1}]`, so both are coupled. Verified against
Python on the by-name-read / string-interp / brace-literal / non-const
adversarial cases; corpus exact 41 → 42.

**O127 forwarding — done.** `forward_candidate` now counts only
non-terminator uses for the single-use gate, matching Python's def-use
chain (which excludes `return $x` / branch-condition terminator reads), so
a computed `set x [cmd]` forwards into its single operand use even when `x`
is also read by a trailing terminator. The inlined `[set x …]` keeps `x`
defined for the terminator read.

**Two propagation miscompiles fixed (2026-06).** Both were the same class
— a constant inlined into a syntactic context that needs quoting:

- **List constant word-split into a string command substitution.**
  `substitute_dollar_refs` inlined a value's raw text into a `"…"` arg,
  rejecting only `$ [ \ "` — not whitespace. A multi-word value (a list
  literal) inside a nested `[…]` split one command argument into several:
  `puts "r: [lsearch -exact $tokens uic]"` with
  `set tokens {tran 1n 100n uic}` became
  `puts "r: [lsearch -exact tran 1n 100n uic uic]"` (original prints `3`;
  rewrite errors `bad option "tran"`). Fixed by tracking command-sub
  nesting depth and requiring a single safe bare word at depth > 0.
- **Whole-word `[cmd]` wrapped as a `"…"` interpolation.**
  `visit_string_interpolation` fired on any non-braced word, wrapping a
  free-standing `[expr {$a + $b}]` in quotes and mis-spanning it
  (`puts "[expr {3 + 4}]"]`). Fixed by skipping whole-word command
  substitutions there (they belong to `visit_call_cmd_subst_folds`).

**`expr "…"` quoted-expr over-fold — fixed (soundness).** `set a alpha;
set b beta; expr "$a == $b"` was folded to `0`, but a quoted/bare expr
substitutes the variable *values* textually before parsing, so tclsh sees
`expr "alpha == beta"` and *errors* (`invalid bareword`). The SCCP
`[expr …]` fold now uses a numeric-only env for the non-braced form
(`sccp::env_from_uses_numeric`): a string-valued var is left unbound so the
fold bails, matching Tcl / Python. Numeric values still fold.

**Braced `[expr {…}]` command-argument fold — done.**
`puts [expr {$a + $b}]` with constants folds to `puts 7`
(`try_fold_expr_with_constants`, braced value-substitution model only — the
quoted form stays conservative).

**Interprocedural argument-sensitive pure-proc folding — done.** A call to a
pure proc with constant arguments now folds to its constant return:
`[::math::add 2 4]` → `6`, passthroughs (`[id 1]` → `1`). Implemented as
`evaluate_proc_with_constants` (port of the Python helper): seed the params
as version-0 lattice constants, re-run SCCP over the callee, resolve a
single constant from all reachable `return` terminators
(`resolve_return_constant` / `fold_return_under_lattice`). **Soundness:** a
simple `$var` return reads the variable's *exit version* precisely, so a
loop-carried `return $total` (a phi → Overdefined) or a recursive
`fibonacci` does **not** mis-fold to a stale pre-loop value; redefined procs
are excluded.

**Remaining follow-ups:**

- The 11 pre-existing over-removals.
- **Rust does less than Python on the core samples** (the bulk of the
  remaining divergences are *not* bugs — Rust is conservative): the
  O117/O120 readability rewrites interact with DCE differently on dead vars
  (a demo-file ordering nuance), append-chain collapse, array-element
  constant propagation (`set who $arr(user)`), and O110 parenthesisation
  canonicalisation (`$part * 100 / $whole` vs `($part * 100) / $whole`).
- Many diverging files under `samples/diagnostics/W*` and
  `samples/for_screenshots/*` are diagnostic/demonstration fixtures whose
  optimiser output is incidental; check them against the
  `_NO_PARITY_KEYS` exclusion before treating as gaps.

The diagnostic-side dead-store/unused parity (W210 / W211 / W220) is closed.

Diagnostic-side dead-store/unused parity (W210 / W211 / W220) is now
closed — see the analyser changes landed alongside this note.
