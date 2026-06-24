# FP catalogue → Rust port: status & bug worklist

Tracks the port of the analyser false-positive precision catalogue
(`docs/design/compiler/FP.md` / `tests/test_fp_*.py`) into the Rust analyser,
per [`fp-rust-port-plan.md`](fp-rust-port-plan.md). Living document.

## Done

The harness and **all 11 families** are ported and committed under
`rust/tcl-compiler/src/analyser/diagnostics/fp/` (`mod fp`). **343 FP tests
pass.** Each test asserts against the live Rust pipeline via the `codes()/fires()`
helper (analyser + `run_all_checks`, and — for O-codes — `optimise_with_dialect`),
mirroring the user-facing `tcl diag` / `get_diagnostics` surface.

| Family | tests | divergences | notes |
|---|---:|---:|---|
| RBS (read-before-set) | 77 | 0 | already had the most Rust coverage |
| STY (style) | 60 | 0 | 1 bridge-only skip (Python lexer-API test) |
| OBJ (object dispatch) | 61 | 7 | snit/TclOO W307/W308 heuristics |
| BND (bounds/intervals) | 21 | 0 | |
| DS (dead-store) | 21 | 1 | FP-DS-04 cross-scope **fixed** |
| SH (shimmer) | 20 | 0 | |
| TNT (taint) | 18 | 0 | |
| NAB (confirm-correct) | 17 | 0 | FP-NAB-03/12 are internal-API, see below |
| INJ (injection) | 12 | 0 | |
| RCH (reachability) | 8 | 0 | drives the optimiser for O107 |
| OPT (optimiser) | 34 | 3 | 2 bridge-only IR-structure skips |

**Fixed bugs the port surfaced**

- **FP-DS-04 (cross-scope traced `::`-global).** A `::w` with a write trace in
  one proc but a `set ::w 1` in another emitted a false `W211`. Added
  `scan_module_traced_globals(cu)` and folded module-wide traced `::`-globals
  into every function's `cross_event_vars` suppression. (commit `b71f8859`)

## Remaining bug worklist (11)

Each is a **genuine Rust analyser/optimiser divergence** the port proved against
the Python verdict, currently a transient `#[ignore]` in the relevant `fp/*.rs`
(to keep the suite green) with the exact divergence in the ignore string. These
are **not** the acceptable-xfail category — each must be fixed so the test passes
and the `#[ignore]` removed. Ordered roughly by tractability.

### Dead-store

1. **FP-DS-02** — false `W220` on `set w 5` in `incr i [expr {$w}]`. The `$w`
   read sits inside the `expr {..}` braces; the substitution-hidden-reads scan
   (`collect_rmw_hidden_reads` / `substitution_hidden_reads_of`) does not treat
   an `expr` command-sub's braced argument as an expression, so the read is
   modelled as neither an SSA use nor a hidden read. Fix: give the cmd-sub read
   scan `expr`-brace awareness (parse the braced arg as an expression), or lower
   `[expr {…}]` arguments so the inner reads become SSA uses.

### Object dispatch (W307/W308) — analyser var-command heuristics

Over-fires (Rust emits where Python suppresses — add a suppression):

2. **FP-OBJ-05** — `W308` on a snit instance method (`$o delegated_or_builtin`).
   snit instances route through delegation/hull/options/built-ins, so method
   validation is unsound; suppress W308 for snit-instance receivers.
3. **FP-OBJ-07** — `W307` on `[cmd]::method` namespaced-ensemble dispatch. The
   literal `::method` tail is static method evidence; suppress.
4. **FP-OBJ-10** (×2) — `W307` on dash-prefixed (`$state(-command)`) and
   callback-suffix (`$state(doneCallback)`) array-element dispatch (callback
   registration slots).

Under-fires (Rust suppresses where Python emits — tighten the heuristic):

5. **FP-OBJ-09** — SCCP-const evidence (`set cmd notacommand; $cmd a; $cmd b`)
   must override the multi-dispatch suppression and fire `W307`.
6. **FP-OBJ-D4-F5** — a local literal dispatched inside an `oo::class` method
   body (`set cmd nope; $cmd arg`) must fire `W307` (the `in_method` blanket
   suppression should not apply without positive object-evidence).
7. **FP-OBJ-VAR-as-cmd** — interproc param-constant seeding should propagate
   non-command evidence so a tainted-as-non-command param fires `W307`.

### Optimiser

8. **FP-OPT-03** — Rust LICM does not hoist an outer-pure/inner-pure nested
   `[format … [expr {…}]]` shape (`O106` not emitted).
9. **FP-OPT-06** — `O100` copy-propagation propagates a stale value past a
   command-sub write (the cmd-sub write is not wired into the SSA kill-sites).
10. **FP-OPT-08** — elimination deletes `set b 0` even though `$b` survives in
    the EXPR role (the EXPR/BODY descent in the overlap filter is not ported).

### Not-a-bug internal-API coverage (Rust-structure tests, not diagnostics)

11. **FP-NAB-03 / FP-NAB-12** — the Python tests assert on the interproc `pure`
    summary and the `is_pure_var_ref` value-shape parser directly (not via a
    diagnostic). Add equivalent Rust unit tests against the corresponding Rust
    APIs (interproc purity; the value-shape / pure-var-ref parser).

## Also deferred to the Rust-structure / LSP-layer track

- **FP-INJ-05 code-action**: the W101→`eval [list …]` quick-fix rewrite is an
  LSP code-action contract; add a test at the `tcl-lsp-core` code-action layer
  (the diagnostic side is covered in `fp/inj.rs`).
- **FP-STY-15 / FP-OPT-12 bridge-only skips**: Python tests that drive
  `TclLexer.tokenise_all()` / `lower_to_ir(...).procedures` IR structure with no
  public Rust surface; behaviourally covered by sibling diagnostic tests.
