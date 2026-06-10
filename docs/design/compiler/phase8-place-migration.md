# Phase 8 (continued) — versioned-Place memory-SSA + coordinated consumer migration

> Status: **partly shipped.** 8A (Place model + resolution + bridge), **8E**
> (refined `overlap`, `e753c95`), and the **headline 8D precision win**
> (array-element + dict-path distinction in dead-store/unused, DCE-consistent,
> `a6fd6fd`) are landed. The array-element distinction was achieved
> *net-negative FPs* (W220 −88 / W211 −2 / 0 added, `make test-opt` green)
> **without** the full 8F versioned substrate — the suppress-only `overlap`
> relation plus 8E's refinement sufficed. 8F is now only needed to *delete* the
> side-channels (cleanup) and to recover rare same-element-reassignment dead
> stores. Remaining: side-channel deletion (8F+8G refactor), 8H analyser bridge,
> 8I residual (INSTANCE_VAR/MRO/snit). See the progress ledger for the live
> status. The sections below remain the reference design for that remainder.

## Why this is a separate, multi-step stage

The original Phase 8 plan assumed the dataflow consumers could switch from
name-string `(name, version)` SSA equality to `place.overlap` in "one
coordinated change". A session attempt at the smallest increment — make
`compiler/core_analyses.py` `_dead_stores` suppress an `ARRAY_ELEM` def when any
function read-place overlaps it — produced **W220 −88, O109 −66, O126 +2** on the
836-file corpus and surfaced **two structural blockers**:

1. **`overlap` is too coarse for dead-store: dynamic-*alias* over-overlap.**
   `gregorian.tcl`'s `set date2(ERA) …` was suppressed not by a `date2` read but
   by reads of an *unrelated* array `date` that is a **dynamic `upvar` alias**.
   `compiler/var_resolve.py` stamps `dynamic=True` on the alias, the array-element
   reads inherit it, and `compiler/place.py:overlap` short-circuits
   `if p.dynamic or q.dynamic: return True` — so one dynamic-aliased array makes
   *every* array-element write in the proc look observed. Sound (suppress-only)
   but it destroys precision wholesale, and it conflates two genuinely different
   notions: a **dynamic index** within a *known* base (`a($i)` — only aliases
   other `a`) vs a **dynamic target/alias** (`upvar 1 $x date` — `date` is a known
   *local symbol* whose *caller* is unknown).

2. **The dataflow `dead_stores`/`unused_variables` lists are shared with the
   optimiser's DCE.** `compiler/optimiser/_elimination.py` consumes
   `analysis.dead_stores` to emit **O109** (`:250`) and keys **O126** off the
   dead-store set (`baseline_dse_keys`, `:306`). Shrinking `dead_stores` in the
   analyser silently unmasked 2 pre-existing **O126** false positives (the
   name-level `unused_variables` can't see the cross-proc `EYMDToJulianDay date2`
   `upvar` read). **Any change to the dead-store set is simultaneously an
   optimiser change** and must be validated by `make test-opt` (VM equivalence),
   not just the diagnostic corpus.

The lesson: precision here requires **per-element versioning** (so `a(k)` ≠
`a(j)` are independent values rather than folding to SSA name `a`), a **refined
overlap** that separates dynamic-index from dynamic-alias, and a **single
reaching-definition substrate shared by the analyser and the optimiser DCE** so
they cannot desync. That is the stage below.

## Current state (what exists, what's missing)

* **`compiler/place.py`** — `Place`/`Index`/`overlap`/`places_read_to_form`.
  `overlap` over-approximates (`dynamic`/`UNKNOWN`/`DYNAMIC`/`ANY` ⇒ `True`).
  Missing: the dynamic-index vs dynamic-alias distinction (blocker 1).
* **`compiler/var_resolve.py`** — `resolve_place(ref, ctx)`; `ResolveContext`.
  Stamps `dynamic` from the upvar-alias target. Sound but coarse.
* **`compiler/place_bridge.py`** — `build_resolve_context` / `def_places` /
  `read_places` / `terminator_read_places`. **Name-level, not versioned** — it
  resolves a statement's def/read *places* but does not pair them with the SSA
  version they write/observe.
* **`compiler/ssa.py`** — `SSAStatement.{uses,defs}` are `dict[name → version]`.
  Array element writes/reads **fold to the base SSA name** (`set a(k) 1` →
  `defs={'a': v}`), so distinct elements share one strong-update version stream —
  the root of the conflation FP.
* **`compiler/memory_ssa.py`** — **half-built.** `MemoryLocationKind` has
  `ARRAY_ELEMENT`/`INSTANCE_VAR`, but `build_memory_ssa` only ever creates
  `LOCAL`/`UNKNOWN` memory ops and only for *aliased* names; it is consumed only
  for clobber/barrier counting. No per-element versioning, no `Place`, no
  overlap-based reaching defs.
* **Consumers** (`compiler/core_analyses.py`): `_dead_stores`,
  `_unused_variables`, `_read_before_set`, `_sccp`/`_escaping_var_names` — all
  name-level SSA + the suppress-only side-channels (`_extra_local_reads`,
  `statement_cmd_sub_write_names`, `dynamic_target_name_reads`,
  `expr_substitution_read_names`, the `$arr($idx)` scan).
* **Optimiser DCE** (`compiler/optimiser/_elimination.py`): O109 from
  `analysis.dead_stores`, O126 from `analysis.unused_variables` minus the
  dead-store keys.
* **Analyser identity** (`analyser/_analyser/_scope.py`, `semantic_model.py`):
  `Scope.variables[base_name] → VarDef` with a flat `array_indices` set; refs /
  rename / hover are name-keyed (8C target).

---

## Stage 8E — Refine `overlap`: dynamic-index ≠ dynamic-alias  *(small, unit-tested, output-equivalent)*

**Problem:** blocker 1 — `dynamic=True` on a known-base array makes it overlap
every other array.

**Change (in `compiler/place.py`):** split the single `dynamic` short-circuit.

* A **dynamic *index*** (`Index.kind is DYNAMIC`) on a *known base name* only
  affects element-vs-element comparison: `a($i)` overlaps any `a(…)` but **not**
  `b(…)`. (Already true structurally — the bug is the `place.dynamic` flag, not
  the index.)
* A **dynamic *alias/name*** (`place.dynamic` from an unresolved `upvar`/computed
  name, or `UNKNOWN`) keeps the over-approximating `True` — but **only against
  places that could share its frame**. Two *distinct* local alias symbols
  (`date`, `date2`) are different lvalues in the current frame; a write to one is
  not observed by a read of the other unless one side is a fully-`UNKNOWN`
  dynamic *name* (`set $X`). So: `overlap(date2(ERA), date($i))` → **False**
  (different base names, neither is a name-level UNKNOWN); `overlap(a(k), set $X)`
  → **True** (UNKNOWN name could be `a`).

**Soundness:** a `upvar 1 $x date` aliases a *caller* var; two different local
alias names aliasing the *same* caller var (`upvar 1 $x date; upvar 1 $x date2`)
is the only way `date`/`date2` truly alias. That requires the *same* `$x` — a
relational fact we don't track. Decide this conservatively but **documented**:
keep `UPVAR_ALIAS`↔`UPVAR_ALIAS` of *different names* as `True` (rare, sound),
but a `dynamic`-flagged **SCALAR/ARRAY_ELEM with a known base name** overlaps
only same-base places. This is the precision the dead-store needs without losing
soundness for the genuine same-caller case.

**Code-review finding — `upvar 0` is same-frame, not an external escape:**
`place_bridge` records every `upvar` local alias as `UPVAR_ALIAS` (escaping)
regardless of level, so `_reportable_local` suppresses W211/W220 for
`proc f {} { upvar 0 x y; set y 1 }` while plain `set y 1` is correctly flagged.
But `upvar 0 x y` aliases a var in the **current** frame (tclsh 9.0.3:
`upvar 0 x y; set y 1` sets local `x`), so `y` is a same-frame alias whose
writes *are* observable locally. The sound precise fix is to resolve level-0
upvar as a **same-frame alias of `x`** (a union-find link `y↔x` in the current
frame, reusing the `memory_ssa` alias substrate) rather than as an external
escape — then `set y 1` with no read of `x`/`y` is a genuine dead store, while
`… ; puts $x` correctly keeps it live. A naive "treat `upvar 0 y` as a plain
local" flip is **unsound** (it would FP when `x` is read by its real name), so
this needs the 8F/`memory_ssa` aliasing layer; keep the current suppress-only
(safe) behaviour until then.

**Code-review finding — upvar caller-def injection ignores the level.**
`_collect_upvar_targets` (`compiler/cfg.py`) records a proc's upvar targets so a
*call site* can inject the caller-side defs the callee writes (`proc h {} {upvar
1 x y; set y 1}` ⇒ calling `h` defines the caller's `x`).  But the target is
recorded for *every* level: `upvar 0 x y` (this proc's own frame), `upvar #0`
(global), and `upvar 2` (grand-caller) all wrongly inject a def into the
*immediate* caller, suppressing its W210/dead-store.  tclsh 9.0.3: `proc h {n}
{upvar 0 x y}; proc c {} {h x; puts $x}` errors `can't read "x"` — `h` does not
define `c`'s `x`.

The naive fix (gate the recording to `level == 1`) is **unsound** because
`_UpvarInfo.literal_targets` is *dual-purpose*: besides caller-def injection it
also feeds **interproc global-write/effect detection** (`upvar #0 ::g x` is how
a proc mutates a global) and is consumed by codegen — gating it to level 1 broke
`test_upvar_global_write_detected_via_cfg` and the VM `upvar.test`.  The sound
fix **separates the two roles**: add a distinct `caller_local_targets` field
(level-1 literal + `$param` targets only) consumed by the call-site def
injection (`_uplevel_set_defs`/`_resolve_upvar_defs`), while `literal_targets`
keeps recording *all* written variables for effect/global-write detection.  A
focused structural change to `_UpvarInfo` + its consumers, gated on the
interproc + VM upvar suites — deferred (it is a false *negative*, lower priority
than the false positives, and entangled with effect detection + codegen).

**Gate:** extend `tests/test_place.py` with an exhaustive overlap table including
the dynamic-index/dynamic-alias/UNKNOWN-name matrix and an over-approximation
property test (any unhandled pair ⇒ `True`). No diagnostic change yet (no
consumer rewired) → corpus byte-identical.

---

## Stage 8F — Versioned-Place reaching definitions  *(the foundation; output-equivalent)*

**Goal:** one substrate — *for each read at a statement, which def-places (with
their versions) reach it under `overlap`* — that both the analyser dataflow and
the optimiser DCE consume, so they cannot desync (blocker 2).

**Change:** promote `compiler/memory_ssa.py` (or a new `compiler/place_ssa.py`)
to compute, over the CFG/SSA in RPO with the existing dominator/phi machinery:

* a **versioned Place store** — every def (scalar, `ARRAY_ELEM`, `DICT_PATH`,
  `INSTANCE_VAR`, alias) gets a fresh version keyed by its `Place`, with
  **strong update** for a scalar / exact-literal element (kills the prior version
  of that exact place) and **weak update** for a dynamic-index element or whole
  array (does not kill sibling elements);
* **clobbers** (`IRBarrier`/`eval`/`uplevel`/dynamic name) bump *all* versions
  (the existing `_is_clobber` logic, generalised);
* a **reaching-def query**: `defs_reaching(read_place, block, idx) → set[(Place,
  version)]` using `overlap` to select observable defs.

Reuse `compiler/place_bridge.py` for per-statement def/read places, but make it
**version-aware**: `def_places`/`read_places` must yield `(Place, version)` by
threading the statement's `SSAStatement.{defs,uses}` versions (and the
reaching-version for `lset`-style read-modify-write and cmd-sub writes).

**Output-equivalent:** *no consumer reads it yet.* Validate the substrate in
isolation:
* unit tests in a new `tests/test_place_ssa.py`: strong vs weak update, sibling
  elements independent, whole-array kills elements, clobber kills all, dynamic
  name = top;
* a **differential** check that for *scalars* the reaching-def result is
  identical to the existing `(name, version)` SSA `used` set (proves no scalar
  regression before any consumer flips).

---

## Stage 8G — Migrate the dataflow consumers + optimiser DCE together  *(output-CHANGING, corpus + VM-equivalence gated)*

This is the "real deletion". Do the analyser consumers **and** the optimiser DCE
in **one coordinated change per consumer**, because they share `analysis.dead_stores`
/ `analysis.unused_variables`.

1. **`_dead_stores`** → a def `(place, version)` is dead iff no read with an
   `overlap`-matching place observes that version (via 8F `defs_reaching`).
   Element writes get weak-update precision (`a(k)` not killed by `a(j)`), fixing
   the conflation FP *without* the 8E-refined overlap over-suppressing. Retire
   `_extra_local_reads` once the versioned read set subsumes per-word deep-only
   (verify the `tcldes.tcl` `lefttemp` deep-only case — finding #1 in the
   ledger).
2. **`_read_before_set`** → version-0 detection over places; retire
   `statement_cmd_sub_write_names` (the cmd-sub writes become real `(Place,
   version)` defs in 8F) and the `$arr($idx)` scan.
   * **Code-review finding (false negatives the current name-level suppression
     causes — verified against tclsh 9.0.3, the proper fix is this 8F/8G
     positioned-def migration, NOT removing the suppressions, which would
     resurrect the idiom FPs they exist to silence):**
     - `command_sub_write_names` / `info exists` / `array exists` are added to a
       **whole-proc** skip set, so a *read before the defining/test statement*
       is wrongly suppressed. tclsh errors on both, we stay silent:
       `proc f {} { puts $msg; set e [catch {error boom} msg] }` (`can't read
       "msg"`) and `proc f {} { puts $x; if {[info exists x]} {…} }` /
       `if {[info exists x]} {…}; puts $x` (`can't read "x"`).
     - Fix: once cmd-sub writes are positioned `(Place, version)` defs (8F), a
       read at an earlier version is naturally read-before-set; the
       `info exists`/`array exists` exemption must be **scoped to reads
       dominated by the test** (i.e. guarded), not proc-wide. Both need the
       version/dominance info only 8F provides — hence tracked here, not patched
       name-level.
3. **`_unused_variables`** → place-grouped; an array used at *any* element marks
   the array used (already true name-level — keep, but source from the shared
   substrate so it agrees with `_dead_stores`).
4. **`_sccp` / `_escaping_var_names`** → `place.is_global or observed or
   dynamic-alias` replaces the name-set; **keep force-OVERDEFINED** (soundness).
5. **Optimiser DCE** (`compiler/optimiser/_elimination.py`) → O109/O126 consume
   the *same* reaching-def substrate (not a parallel name-level view), so the
   analyser and optimiser cannot disagree (kills the O126 +2 desync).

**Gates (mandatory, per consumer):**
* `bench/diag_dump.py` corpus delta **audited sample-by-sample** — every removed
  W210/W211/W220 confirmed a genuine FP (overlap-justified), every *added* one
  tclsh-justified; **net-positive only**.
* **`make test-opt`** (VM equivalence) green — proves no unsound O109/O126
  elimination (the blocker-2 risk).
* `make test-py` green; `tests/test_bytecode_identity.py` green (Places are
  analysis-only — codegen untouched).
* W210 exact-count gate (historically the painful seam): `idna.tcl:233 set w 1`
  stays clean; no read-before-set regression.

**Back-off rule:** if a consumer can't be made net-positive + VM-equivalent,
revert that consumer and record the residual side-channel it still needs.

---

## Stage 8H — Analyser bridge (8C)  *(LSP-feature-scoped; diagnostics output-equivalent)*

**Change:** `VarDef.place` (`analyser/semantic_model.py`); `_define_var` /
`_record_var_read` (`analyser/_analyser/_scope.py`) group by `Place` so the
analyser and compiler share one identity. Reuse `_resolve_qualified_var` /
`_namespace_from_scope` as the analyser's `ResolveContext` source.

**Scope decision (document, don't guess):** array-element **rename** still
renames the whole array `a` (renaming `a(k)` occurrences only is not what users
want); Place grouping is for **find-references precision** (distinguish `$ns::v`
vs local `v`; qualified-vs-`variable`-declared identity) and for making W210/refs
agree with the SSA path. Keep element-rename name-level.

**Gate:** LSP golden tests (`tests/test_*references*`, rename, hover); byte-identical
diagnostics; the `lsp-client` skill e2e for references/rename/hover.

---

## Stage 8I — Precision wins (8D)  *(output-CHANGING; per-finding tclsh-audited)*

Built on 8F's versioned places; each behind a net-positive `bench/diag_dump.py`
audit + `make test-opt`:

* **Array-element index distinction** in dead-store/unused (`a(foo)` ≠ `a(bar)`)
  — the headline precision win, now sound because 8E+8F give weak-update
  versioning instead of name-folding.
* **Dict-path decomposition** (`dict set d a b` vs `dict get d a`).
* **`INSTANCE_VAR`** populated from `variable` / `my variable` in TclOO methods
  (`analyser/_analyser/_oo.py`, `semantic_model.py`); `ClassDef.variables: list`
  → `instance_vars: dict[str, InstanceVarDef]`.
* **`my`/`self`/`next`/MRO** — a `MethodResolution` helper over the existing
  superclass/mixin chain (OO analogue of command canonicalisation).
* **snit** — `_handle_snit` lowering `variable`/`option`/`typevariable` into the
  Place model instead of opaque barriers.
* Residual gap-C/gap-D (`[set x …]` cmd-sub assignment *defs* into SSA;
  custom proc-definer param seeding) as Place-def operations.

---

## Cross-cutting

* **Explorer:** a `places` view (per-statement def/read places + versions) and a
  `memory-ssa` / reaching-defs overlay on `ssa`; surface element-distinction in
  the `dataflow` view. (Cross-cutting visualisation item P-8.)
* **Invariants (must hold throughout):** Places are analysis-only — default
  bytecode stays tclsh-byte-identical (`tests/test_bytecode_identity.py`); `_sccp`
  force-OVERDEFINED on global/escaping/observed; `overlap` over-approximates
  (suppress-never-flag) until 8I tightens with audits; the analyser dataflow and
  optimiser DCE consume the **same** reaching-def substrate.

## Sequencing & dependency order

```
8E (overlap refine, unit-tested, byte-identical)
  └─ 8F (versioned-place reaching defs, output-equivalent foundation)
       ├─ 8G (consumer + optimiser DCE migration, net-positive + test-opt)   ← retires side-channels
       └─ 8H (analyser bridge / 8C, LSP-gated)        ← independent of 8G, can parallel
            └─ 8I (precision wins / 8D, per-finding audited)                  ← needs 8F + 8G
```

8E and 8F are **safe** (byte-identical / output-equivalent) and should land
first to de-risk. 8G is the high-stakes coordinated migration (corpus +
VM-equivalence gated, per-consumer back-off). 8H is independent and low-risk. 8I
is the output-changing payoff, gated per finding.

## Effort & risk summary

| Stage | Risk | Gate | Notes |
|---|---|---|---|
| 8E overlap refine | low | unit + byte-identical corpus | unblocks precision |
| 8F versioned places | medium | unit + scalar-differential | output-equivalent foundation |
| 8G consumer+DCE migration | **high** | corpus net-positive + `make test-opt` + bytecode identity | per-consumer back-off; retires side-channels |
| 8H analyser bridge (8C) | low–med | LSP golden + byte-identical | independent |
| 8I precision (8D) | med (output-changing) | per-finding tclsh audit + `make test-opt` | the payoff |
