# Rust optimiser parity snapshot

A scan run under `TCL_LSP_RUST_OPTIMISER=1` against the three
optimiser test files establishes the current parity gap between
the Python and Rust optimiser pipelines.

```bash
TCL_LSP_RUST_OPTIMISER=1 uv run pytest \
  tests/test_optimiser.py \
  tests/test_optimiser_coverage.py \
  tests/test_optimiser_vm_equivalence.py \
  -q --tb=no
```

Result after the parity-closing work in this session:

```
209 failed, 487 passed  (down from 338 / 357)
```

Total: 129 tests fixed (38% reduction from the initial snapshot).

## Parity commits landed in this strip

| Commit               | Area                                         | Δ tests |
|----------------------|----------------------------------------------|--------:|
| `C30-operand-spans`  | O102 / O103 applicable rewrites (Strip A)    | baseline |
| `cat3`               | Span truncation fix (`full_rewrite_span`)    | -8      |
| `cat4-cat5`          | Braced-literal guards + `call` indirection   | -4      |
| `cat3-quoted-strings`| Composite ``"…"`` word span extension        | -14     |
| `cat3-extended`      | `full_rewrite_span` across remaining passes  | -4      |
| `cat2-assign-expr`   | Constant fold + instcombine on `AssignExpr`  | -25     |
| `cat2-instcombine`   | 11 new identity / absorbing rewrites         | -22     |
| `cat2-expr-wrapper`  | Collapse ``[expr {K}]`` when `K` is bare     | -11     |
| `cat2-unary-self`    | Unary identity + inversion + self-compare    | -26     |
| `cat1-cat2-propagation` | Cascade SCCP constants into `AssignExpr` exprs | -4 |
| `cat2-ternary-demorgan` | Ternary constant fold + DeMorgan laws     | -11     |

## Remaining failure clusters (209 total)

Top five categories sorted by count:

| Test class                          | Fails | Nature                         |
|-------------------------------------|------:|--------------------------------|
| `TestOptimiser`                     |    38 | Multi-pass cascade / DSE       |
| `TestO110InstCombine`               |    17 | Reassociation (`$a + 1 + 2`)   |
| `TestTailCallOptimisation`          |     9 | Return-value shapes            |
| `TestConstantVarRefPropagation`     |     9 | Cascading DSE after propagation|
| `TestPatternMatchSimplification`    |     8 | `switch` / pattern matching    |

### 1. Cascading DSE / multi-pass interaction (largest remaining gap)

The Python pipeline iterates passes to fixpoint:
propagation → folding → DCE → more propagation. Rust runs each
pass once. A source like:

```tcl
set x 42
puts $x
```

emits O102 (propagate `$x → 42`) on the `puts`, but the now-dead
`set x 42` is not removed. The Python side's `find_optimisations`
wraps the emission in an iteration loop that re-runs SCCP / DCE
on the rewritten IR until no new rewrites fire.

Fixing this requires implementing a fixed-point loop in
`optimiser::manager::optimise_with_dialect` — apply the
rewrites to a copy of the source, rebuild the CU, re-run the
pass pipeline, repeat until no new rewrites appear (with a
sensible iteration cap). This is the single highest-impact
remaining change — would close `TestOptimiser::test_chained_*`,
`TestConstantVarRefPropagation`, and several `TestPassInteractions`.

### 2. O110 reassociation (17 fails)

The remaining `TestO110InstCombine` failures are all
reassociation patterns: `$a + 1 + 2` → `$a + 3`, `$a * 2 * 3`
→ `$a * 6`, `$a + 3 - 1` → `$a + 2`. The Rust `strength_reduce_node`
handles local identities but not constant reassociation across
a chain of binary operators. Follow-up: implement a
constant-coefficient reassociation pass (sort children of a
left-associative chain into constant-subtree / variable-subtree
and fold the constant side).

### 3. iRules-specific call-graph edges (7 fails)

`TestUnusedIruleProcs` still has 7 failures where Python sees
transitive reachability the Rust side misses. The basic `call
<proc>` indirection is fixed; remaining gaps involve namespace
resolution on qualified names and `RULE_INIT`-as-library
detection edge cases.

### 4. String-interpolation edge cases (8 + 6 fails across two classes)

`TestO120StringCompareEqNe` and `TestO117StrlenZeroCheck` need
the full Python-wrapper treatment applied to
`[string length …]` substitution detection and `==` / `!=`
promotion in branch-condition context.

### 5. Spurious rewrites (varied)

A handful of tests where Rust produces an optimisation Python
doesn't — usually because Rust is missing a guard that Python
has (cross-event variable writes, iRules event boundaries, etc.).

## Status

`TCL_LSP_RUST_OPTIMISER` remains **opt-in**. Flipping the default
still requires closing ~209 tests, dominated by the cascading-loop
issue (~60+ tests). Recommended next step: land the fixpoint
iteration in `optimiser::manager` — a single change that unlocks
most of the remaining cluster.

Cosmetic message wording continues to match Python byte-for-byte
across spot checks (O100 / O101 / O102 / O103 / O104 / O110 /
O113 / O121 / O122 / O124) — no wording-only patch is warranted.
