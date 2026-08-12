# KCS: Array elements are per-key SSA variables (typing, folding, shimmer)

> **Audience:** Contributor
> **Type:** How-To

## Applies to

all-editors, ssa, sccp, type-infer, shimmer

Covers `rust/tcl-compiler` SSA construction, SCCP, type inference, shimmer,
and the dead-store/unused diagnostics, plus `rust/tcl-lsp-core` hover.

## Symptom

- A diagnostic or hover claim about `arr(k)` looks wrong: a type/constant
  seems to leak between elements, a dead-store fires (or goes silent) on an
  array element, or an S100/S101/S102 involves `arr(...)` names.

## Operational context

- A **constant-keyed** element (`arr(k)`;
  `set {a($x)} v`'s literal `$x`; `${arr($i)}`'s literal `$i`) is its own
  SSA variable named `base(key)` (`tcl_syntax::naming::element_var_name`).
  A **dynamic** key (`arr($i)`) stays on the conflated base symbol.

## Decision rules / contracts

1. Element symbols type, const-fold, and shimmer-check independently —
   the oracle rule is "array elements behave as independent scalars"
   (tclsh-verified in `docs/design/compiler/type-tracking.md`).
2. A dynamic-key or whole-array write **fans** over the array's known
   elements as `SsaStatement::may_defs`. A may-def is a *may*-write: its
   SCCP value and type are the **join** of the prior version (recorded as
   a use on the same statement) with the written value — never the
   written value alone.
3. An element write also defs the base as a valueless may-def (whole-array
   readers like `array get` see a fresh version; `$arr` never folds or
   types as an element's value).
4. Write-sensitive passes (W211/W220/O109/O126) must skip synthetic defs
   — check `SsaFunction::is_synthetic_def`. The user wrote one
   assignment, not one per fanned symbol.
5. Base-keyed policy sets (special vars such as `env`, traced vars,
   scope-alias/`variable` tails, instance/cross-event state) are consulted
   through the element's **base** (`normalise_var_name` of the SSA name).
6. W210 read-before-set stays silent for an element whose base is
   written, aliased, or a parameter anywhere in the function; only a
   wholly-unwritten, unaliased array's element read reports.
7. The FP-SH-13 exclusion set (`array_element_symbols`) contains only
   **base** symbols. Do not add element symbols to it — independent
   elements cannot conflate structurally, and the same-element oscillation
   is a genuine S102.

## Repro workflow

1. Dump the SSA to see the symbols and may-defs in play:
   `cargo run -p tcl-cli --bin tcl -- explore --show ssa --text fixture.tcl`
   (element defs render as `arr(k)#2=...`; fanned/base may-defs appear in
   the same statement's `defs`).
2. Lock behaviour with a focused test at the right layer:
   type inference (`type_infer::tests::array_elements_type_independently`,
   `dynamic_key_write_joins_element_types`), diagnostics
   (`rust/tcl-compiler/tests/checks.rs` dead-store/RBS cases), or e2e
   (`diagnostic_matrix.rs` array rows).

## Failure modes

- Treating a may-def like a real def (missing `is_synthetic_def` skip)
  duplicates W211/W220-class reports per fanned symbol.
- Assigning a statement's value uniformly to every def re-opens the
  wrong-element fold (`return $arr(a)` folding to a dynamic write's
  value) — the join rule exists precisely to prevent this.
- Checking a base-keyed policy set with the element-qualified name makes
  `set env(FOO) x` (and traced/instance elements) falsely reportable.

## Discoverability

- Linked from `docs/kcs/README.md`; design contract in
  `docs/design/compiler/type-tracking.md` §"Arrays as per-constant-key
  scalars" and `docs/design/contracts/shimmer-reference-behaviour.md`.
  The base-symbol exclusion cases (FP-SH-13) are pinned by the tests in
  `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs`.
