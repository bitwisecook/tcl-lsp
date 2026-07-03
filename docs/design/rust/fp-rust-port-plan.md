# FP precision suite → Rust port plan

> **Update (2026):** Python is fully retired on this branch, so the
> Rust analyser's standalone FP precision net (the goal of this plan) is
> now the *sole* net — the "re-establish a Python↔Rust differential in
> CI" half is moot, since there is no Python side left to diff against.
> This document is retained as the port plan and precision-contract
> record; treat its CI-differential and "remaining families" steps as
> the mid-flight plan, not the current state.

Concrete plan to give the **Rust** analyser a standalone false-positive (FP)
precision net mirroring the Python FP catalogue, and to re-establish a
Python↔Rust differential parity check in CI — so retiring Python does not delete
the precision contract.

Companion docs: [`../compiler/FP.md`](../compiler/FP.md) (the catalogue),
[`cleanup-status.md`](cleanup-status.md) (closes the "FP test net" P0 item when
this lands), [`../../rust-rewrite-test-audit.md`](../../rust-rewrite-test-audit.md)
(whose "`test_fp_*` → Ported" row this plan corrects — see §9).

## 1. Why

The analyser's precision contract lives in `docs/design/compiler/FP.md`: **113
catalogued determinations** across 11 families, each pinned by a paired
*must-fire* (TP) and *must-stay-silent* (FP) test. The Python suite encodes them
as **360 functions** in `tests/test_fp_*.py`, run unconditionally in CI against
the **pure-Python** analyser.

The **Rust** analyser is defended by a *separate, thinner* set: only **~15 of
113 ids (~13%)** have a genuine Rust assertion. Whole families are at ~zero:

| Family | ids | genuine Rust tests | Python-only |
|---|---:|---:|---:|
| OBJ (object dispatch W307/W308, snit) | 18 | 0 | 18 |
| OPT (optimiser O1xx) | 12 | 0 | 12 |
| STY (style W126/W3xx) | 16 | 3 | 13 |
| INJ (injection W101/W105/W301/T102) | 5 | 0 | 5 |
| BND (bounds/intervals) | 6 | 0 | 6 |
| RCH (reachability O107) | 4 | 0 | 4 |
| DS (dead-store W211/W220) | 9 | 2 | 7 |
| SH (shimmer S100/S101/S102) | 8 | 1 | 7 |
| NAB (confirm-correct) | 12 | 1 | 11 |
| TNT (taint T100) | 5 | 1 | 4 |
| RBS (read-before-set W210/W213/W214) | 18 | 7 | 11 |
| **Total** | **113** | **~15** | **~98** |

And **no Python↔Rust differential runs in CI at all** — the old C41 analyser
differential and the signature-scan differential were *deleted* (commit
`07c1b4d4`, #372) when their Python oracle symbol (`_materialise_rust_analysis`)
was removed; nothing replaced them. So a Rust-analyser regression away from the
Python verdict is caught only where someone hand-ported the specific case.

When Python is retired (the stated go-forward), the 360-test precision net and
the only cross-check both disappear. This plan replaces them with a Rust-native
net plus a transitional differential.

## 2. The good news: this is a missing-*tests* gap, not missing-*features*

The underlying analyser behaviour the catalogue exercises **already exists in
Rust**, so the bulk of the work is writing assertions, not porting logic:

- snit modelling: `handle_snit_type_command` (`analyser/commands.rs:373`); a snit
  W307 suppression test already exists (`diagnostics/tests.rs:2665`).
- every catalogued code has a Rust emitter: W307 (14 sites), W308 (12), O107 (8),
  W101/W105 (4 each), W301 (5), T102, W126 (2), W201 (4), S100 (6).

Exceptions that need a small **logic** addition before their test can pass
(Category B — do these in Phase 3):

| id | gap | where |
|---|---|---|
| FP-OBJ-10 | no callback-shape `$state(-command)` array-element heuristic | analyser var-command path |
| FP-OBJ-08 | W101/W307 dedup only partial | `diagnostics/usage.rs:58` |
| FP-STY-04 | W126 `lassign` var-write suppression exists but isn't wired into a tested path | `dataflow.rs:1461`, `param_traits.rs:469` |
| FP-DS-04 (cross-scope) | `scan_scope_aliases` is per-CFG-function, so a namespace-global `::w` traced in one proc and written in another still fires W211 | `optimiser/elimination.rs:1038` |

Everything else is Category A (feature present, test missing).

## 3. Two tracks

1. **Native Rust FP suite (durable).** Port each FP entry to a Rust `#[test]`
   asserting the *known, fixed* verdict from FP.md. Standalone — survives Python
   retirement. This is the real fix.
2. **Rust↔Python differential (transitional bridge).** Drive the FP reproducers
   through *both* analysers and diff the catalogued codes, as a both-directions
   drift net until Python retires. Catches a Rust regression *and* a Python one.

## 4. Track 1 — native Rust FP suite

### 4.1 Layout

One file per family, mirroring the Python split, beside the existing inline
tests:

```
rust/tcl-compiler/tests/fp/mod.rs        # shared harness (codes_for_dialect, fires)
rust/tcl-compiler/tests/fp/obj.rs        # FP-OBJ-01 … -18
rust/tcl-compiler/tests/fp/opt.rs
rust/tcl-compiler/tests/fp/sty.rs
... (one per family)
```

(Integration-test files keep the catalogue out of the already-large
`diagnostics/tests.rs`. Alternatively extend that module — the helpers there are
the template either way.)

### 4.2 Shared harness

Generalise the existing private helpers (`codes_for` at
`diagnostics/tests.rs:2392`, `w210_fires_for` at `:1863`) into a dialect-aware,
`run_all_checks`-complete pair exposed for integration tests:

```rust
// fp/mod.rs — mirrors Python tests/test_fp_*.py::_codes
/// All diagnostic codes the full pipeline emits for `src` under `dialect`
/// (analyser + compiler-checks, matching Python `analyser.analyse`).
pub fn codes_for_dialect(src: &str, dialect: &str) -> Vec<String> {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(src, dialect);            // analyser diagnostics
    let mut codes: Vec<String> =
        r.diagnostics.iter().map(|d| d.code.as_str().to_string()).collect();
    // plus the SSA/shimmer/taint compiler-checks Python's analyse() integrates:
    codes.extend(run_all_checks_codes(src, dialect));
    codes
}
pub fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes_for_dialect(src, dialect).iter().any(|c| c == code)
}
```

> **Layering note (the trap that killed the C41 differential).** Python's
> `analyser.analyse` integrates `run_compiler_checks` (dead-store W110/W220,
> shimmer S1xx, taint T1xx), but the Rust `Analyser` is analyser-only — the
> compiler-check codes come from a separate path (`run_all_checks`, the same one
> `tcl diag` drives, `tcl-cli/src/commands/diag.rs:171`). `codes_for_dialect`
> must union both, or FP entries for S/T/W110/W220 codes will spuriously "pass"
> by never emitting. Classify each catalogued code by emitting pipeline up front.

### 4.3 Per-entry template (worked: FP-OBJ-01)

Snippets are copied **verbatim** from the Python test (the Python files already
guarantee they match FP.md; for entries whose FP.md reproducer is a placeholder
— e.g. OBJ — the Python test is the source of truth):

```rust
// fp/obj.rs — pairs to tests/test_fp_obj.py::test_FP_OBJ_01_*
use super::fp::{codes_for_dialect, fires};

#[test]
fn fp_obj_01_snit_self_references_no_w307() {
    // FP-OBJ-01: $self/$type/$selfns/$win inside a snit::type method body
    // dispatch on the current object — must NOT fire W307.
    for r in ["self", "type", "selfns", "win"] {
        let src = format!("snit::type T {{\n method m {{}} {{ ${r} foo }}\n}}");
        assert!(!fires(&src, "tcl8.6", "W307"), "${r} foo in snit body fired W307");
    }
}

#[test]
fn fp_obj_01_self_ref_outside_snit_still_w307() {
    // TP control: same names in a vanilla proc ARE stray non-literal dispatch.
    for r in ["self", "type", "selfns", "win", "hull"] {
        let src = format!("proc f {{}} {{ set {r} [getThing]\n ${r} foo }}");
        assert!(fires(&src, "tcl8.6", "W307"), "${r} foo outside snit must warn");
    }
}
```

Each Rust test: carries the `FP-<id>` in a comment, asserts the FP (silent) and
the TP (fires) arm, threads the FP.md-specified dialect.

### 4.4 Drift guard

Add one meta-test asserting the Rust suite covers every catalogued id:

```rust
#[test]
fn every_fp_id_has_a_rust_test() {
    // FP_IDS parsed from docs/design/compiler/FP.md headers at build time
    // (include_str! + regex); fail listing any id with no fp_<fam>_<nn>_ test.
}
```

This is the Rust analogue of the Python suite's completeness and prevents the
catalogue growing without Rust coverage (the exact failure that let `ledit`
ship — see cleanup-status §"registry-completeness gate").

## 5. Track 2 — Rust↔Python differential CI job

### 5.1 Driver

`bench/fp_snippets.py` already enumerates every id and renders its reproducer
(`python -m bench.fp_snippets --id FP-RBS-03`; `main()` at `:3458`). Add a
`--differential` mode that, for each id:

1. **Python** — `from analyser import analyse`; collect diagnostics on the
   reproducer, filter to the entry's catalogued codes.
2. **Rust** — shell `tcl diag --json --dialect <d> <reproducer>` (`run_diag`,
   which runs the full `run_all_checks`, matching Python's integrated set);
   parse the JSON, filter to the same codes.
3. **Assert** verdict parity: for an FP entry the code is **absent on both**; for
   a TP entry it is **present on both**; and the per-code presence set agrees.

Use `tcl diag` (not the analyser-only PyO3 `analyse_tcl` facade,
`facades.rs:129`) as the Rust oracle, for the §4.2 layering reason. Expose the
reproducer + dialect + codes that already live in each `fp_snippets` entry
object so the harness needs no second catalogue.

### 5.2 CI wiring — fail, not skip

New job in `.github/workflows/ci.yml` and `rust-gate.yml`:

```yaml
  fp-differential:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - name: Build the tcl CLI            # hard prerequisite — no silent skip
        run: cargo build -p tcl-cli --release
      - name: FP catalogue Python↔Rust parity
        run: uv run --extra dev python -m bench.fp_snippets --differential
          # exits non-zero on any verdict mismatch; PATH includes target/release
```

The old differentials skipped when their oracle was absent; this one **builds**
its Rust oracle as a step, so absence is a build failure, not a skip. Add it to
the required-checks set (the `rust-gate` equivalent).

## 6. Phasing

| Phase | Work | Output |
|---|---|---|
| **0 — harness** | `fp/mod.rs` helpers; `--differential` driver; CI job — proven green on the ~15 already-covered ids | end-to-end pipeline, no new coverage yet |
| **1 — live bug** | Fix FP-DS-04 cross-scope `W211` (make the traced-global set module-wide, not per-CFG); add its Rust FP+TP test | the one catalogued FP that still fires today is closed |
| **2 — bulk port (Cat A)** | Port zero-coverage families by value: OBJ 18, OPT 12, STY 13, BND 6, INJ 5, RCH 4, then DS/SH/NAB/TNT/RBS remainder | Rust FP coverage → ~109/113 |
| **3 — feature gaps (Cat B)** | Add the small logic for FP-OBJ-10, OBJ-08, STY-04; then their tests | Rust FP coverage → 113/113 |
| **4 — enforce** | `every_fp_id_has_a_rust_test` on the gate; differential job required; correct `rust-rewrite-test-audit.md` + close cleanup-status FP item | regression-proof |

## 7. Effort

~113 entries × (FP + TP) ≈ **~200 Rust test functions**, plus the harness and
driver. Mechanical once Phase 0 lands and the first family sets the pattern.
Rough estimate: harness/driver/CI **2–3 days**; porting **~10–15 entries/day** →
**~2 weeks** for the catalogue; Cat-B logic **2–3 days**. **~3 weeks total**,
parallelisable by family (each `fp/<family>.rs` is independent).

## 8. Risks & mitigations

- **Layering mismatch (analyser-only vs compiler-checks-integrated)** — the bug
  that sank C41. *Mitigation:* drive the Rust side through `tcl diag` /
  `run_all_checks` and union both code sources in `codes_for_dialect` (§4.2);
  pre-classify each catalogued code by emitting pipeline.
- **Dialect sensitivity** — many entries vary by Tcl 8.4/8.5/8.6/9.0.
  *Mitigation:* thread the FP.md-declared dialect per entry; for "varies" entries
  assert per-dialect.
- **Snippet drift between FP.md, the Python test, and the Rust test.**
  *Mitigation:* single source — the differential asserts the Python-test snippet
  reproduces the FP.md reproducer; the Rust test copies the same string with an
  `FP-<id>` anchor.
- **Placeholder reproducers** (OBJ et al. note "outside the per-proc snapshot").
  *Mitigation:* take the real snippet from the Python test, not the FP.md
  placeholder block.

## 9. Correcting the audit record

`docs/rust-rewrite-test-audit.md` classifies `test_fp_*` as **"Ported — ~1,000
analyser `#[test]`s incl. per-code fire/suppress families."** That is true at the
level of *per-diagnostic-code* coverage but **overstated at the level of the
precision catalogue**: only ~13% of the 113 FP determinations have a genuine
Rust assertion, and no differential enforces parity. When this plan's Phase 2
lands, update that row to distinguish *per-code* coverage (already broad) from
*per-catalogue-entry* coverage (delivered here), and flip the cleanup-status FP
P0 item to done.

## 10. Acceptance

- Every FP-id has a Rust FP+TP test; `every_fp_id_has_a_rust_test` green → 113/113.
- `bench.fp_snippets --differential` green in CI, **fail-not-skip**, on the
  required gate.
- FP-DS-04 cross-scope `W211` no longer fires; regression-tested.
- `cleanup-status.md` "FP precision Rust test net" item closed;
  `rust-rewrite-test-audit.md` `test_fp_*` row corrected.
