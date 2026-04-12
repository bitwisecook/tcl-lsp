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

Result (as of this commit):

```
339 failed, 357 passed
```

## Top failure classes

By test module / class:

| Module                                | Class                        | Fails |
|---------------------------------------|------------------------------|------:|
| test_optimiser_coverage.py            | TestO110InstCombine          |    59 |
| test_optimiser.py                     | TestOptimiser                |    57 |
| test_optimiser_coverage.py            | TestO101ConstantFolding      |    21 |
| test_optimiser_coverage.py            | TestO120StringCompareEqNe    |    10 |
| test_optimiser.py                     | TestTailCallOptimisation     |    11 |
| test_optimiser.py                     | TestConstantVarRefPropagation|     9 |
| test_optimiser_coverage.py            | TestO100ConstantPropagation  |     9 |
| test_optimiser_vm_equivalence.py      | TestMultiPassInteraction     |     8 |
| test_optimiser.py                     | TestUnusedIruleProcs         |     8 |
| test_optimiser.py                     | TestPatternMatchSimplification|    8 |
| test_optimiser.py                     | TestCodeSinking              |     8 |
| test_optimiser.py                     | TestStringCompareEqNe        |     7 |
| test_optimiser_coverage.py            | TestPassInteractions         |     7 |
| …                                     | …                            |   … |

## Categorised gaps

The failures split into five categories. Each category is its own
follow-up strip; none are cosmetic. Fixing them requires real
semantic work on the Rust side — message / wording differences are
rare and already line up.

### 1. Cascading analysis loops

The Python pipeline iterates optimisations to fixpoint, letting
propagation enable folding, folding enable DCE, DCE enable more
propagation, and so on. The Rust pipeline runs each pass once and
stops — so a source like:

```tcl
set a 1
set b [expr {$a / 2}]
```

gets `$a → 1` substituted into the `expr` arg (O100 fires) but
the `expr {1 / 2}` never folds to `0`, and the unused `set a 1`
is never eliminated.

Representative failures:
- `TestOptimiser::test_division_now_folds`
- `TestOptimiser::test_propagates_and_folds_expr`
- `TestConstantVarRefPropagation::test_all_uses_propagated_allows_full_dse`

### 2. Missing optimisation bodies

Several pass bodies are stubbed or partially ported on the Rust
side; the missing functionality is listed by O-code:

- **O110 InstCombine** (59 fails) — reassociation, constant 0/1
  identities, `x * 0 → 0`, `x ** 2 → x * x` beyond the minimal
  cases in `optimiser::expr_simplify`.
- **O120 / O117** — full strength-reduction / strlen / eq-ne
  compare simplification.
- **O127 Load Forwarding** (`TestLoadForwarding`) — full copy
  propagation of the form `set x $arg; puts $x` → `puts $arg`
  isn't emitted; the Rust `optimise_load_forwarding` currently
  forwards only literal definitions.
- **O123 Accumulator tail-call hints** — partially landed.

### 3. Span truncation on applicable rewrites

The lexer's representative-token span for `${name}` and `[cmd …]`
words omits the closing `}` / `]`. Several passes emit applicable
rewrites against this span — when materialised, the rewrite
leaves an orphan `}` / `]`:

```tcl
# Source (pre-rewrite):
if {$b == 0} { return $a } else {
    tailcall gcd $b [expr {$a % $b}]
}

# Rust-optimised (post-rewrite):
tailcall gcd $b [expr {$a % $b}]]    # ← orphan `]`
```

Strip A (C30-operand-spans) fixed this for the O102 / O103
rewrites in the propagation pass via a
`full_word_span(source, argv_span)` helper that extends the span
by one byte when the next source byte is the matching closing
delimiter. The same fix is needed in `optimiser::structure_elimination`
and `optimiser::tail_call` (at minimum — a grep for
`argv.span()` / argv-span-based rewrites across the optimiser
module would find every call site).

Representative failures:
- `TestMultiPassInteraction::test_const_prop_into_fold_into_dce`
- `TestTailCallOptimisation::test_o122_output_is_valid_tcl`
- `TestTailCallOptimisation::test_tail_call_in_switch_arm`

### 4. Reachability / call-graph differences

`optimiser::unused_procs` (O124) fires for procs that are
transitively reachable from events in the Python analysis but
unreachable in the Rust analysis. The Python side resolves
`call proc_name …` and namespace-qualified calls differently.

Representative failures:
- `TestUnusedIruleProcs::test_used_proc_not_commented_out`
- `TestUnusedIruleProcs::test_transitively_used_proc_not_commented_out`
- `TestUnusedIruleProcs::test_qualified_proc_call_counts_as_used`

### 5. Spurious output

A handful of failures are cases where the Rust side emits an
optimisation Python does not — almost always because the Rust
side lacks a guard that Python has (e.g. the `eval` / `upvar`
barrier detection in O124, or the braced-literal guard in O121 /
O122 / O123).

## Cosmetic messages: parity already holds

Spot checks on message wording (`Propagate constant into command
argument`, `Forward literal load of '…'`, `Fold pure-proc call to
'…'`, `Proc '…' is not called from any event`, `Fold return of
constant variable`) all match Python byte-for-byte. No
wording-only patch is warranted.

## Status

`TCL_LSP_RUST_OPTIMISER` remains **opt-in**. Flipping the default
to Rust requires closing the five categories above. Each should
be its own commit (or strip) to keep the review / revert surface
manageable. Start with **category 3** (span truncation) — the
smallest, most mechanical change, and prerequisite for any VM
equivalence test passing through the Rust pipeline.
