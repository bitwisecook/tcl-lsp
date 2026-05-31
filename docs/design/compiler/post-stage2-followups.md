# Post stage-2 architectural follow-ups

This doc tracks the **4 PARTIAL closures** left after the stage-2 review wave
(branch `claude/stage-2-fp-catalog`).  Each PARTIAL shipped the
*soundness/precision gate* the reviewer asked for, but its full closure
depends on an upstream architectural change that does **not** belong on
the stage-2 branch.  They are grouped here by the architectural
prerequisite so a future agent can pick up the *whole* prereq (with its
acceptance criteria + corpus targets) rather than fighting the
data-only edge of each PARTIAL one at a time.

Companion docs:

- [`review-findings-tracker.md`](review-findings-tracker.md) — the
  per-finding ledger (rows tagged 🔄 PARTIAL link here).
- [`FP.md`](FP.md) — the verdict catalog (the shipped gates are pinned
  at `FP-OBJ-14`, `FP-OBJ-15`, `FP-OPT-12`).
- [`review-findings-deferred.md`](review-findings-deferred.md) — the
  broader deferred-findings ledger (cross-referenced where the topics
  overlap).

Ground truth: real C tclsh 9.0.3 throughout.

---

## Status snapshot at merge

- ✅ **46 FIXED** (full closure, with regression test + FP.md entry).
- 🔄 **4 PARTIAL** — covered by this doc.
- ⬜ **0 TODO**.

The 4 PARTIALs share **two** architectural prerequisites:

| Prereq | Blocks |
|---|---|
| **A.** VAR-as-cmd type inference (track per-SSA-version the literal command-name a variable holds) | D3-P5, D4-F6, SF-1 |
| **B.** TclOO method-body lowering to per-method `FunctionUnit`s + interproc summaries | SF-2 |

Both prereqs are properly the remit of the algorithmic-improvements
plan on `claude/parser-compiler-algorithms-9FN6G`, **not** the stage-2
branch.  They are listed here so they don't get lost.

---

## A. VAR-as-cmd type inference

### Problem (verified vs tclsh)

```tcl
proc parse {input} { ... }
proc dispatch {} {
    set cmd parse           ;# $cmd now holds the literal "parse"
    $cmd "5 + 3"            ;# fires W307 — analyser can't see $cmd's value
}
```

The analyser flags `$cmd "5 + 3"` as a non-literal dispatch (W307)
even though the literal command name is statically known at that
program point.  Real tcllib code hits this idiom in
`pt::parse::peg`, `grammar::me`, `struct::tree walk`, and many
package dispatcher modules; the residual W307 census after stage-2
(see `bench/diag_dump.jsonl`) found:

- **4 714 / 4 755 (99 %)** of residual W307 firings are
  variable-as-command dispatch (`$var arg…`).
- **35** are cmd-sub-as-command (`[expr] arg…` style); most already
  have correct `return_type` in the registry and would be picked up
  once the analyser actually consults them through the use chain.

### Why registry data alone can't close it

The reviewer's suggestion in D3-P5 (add `return_type` to specific
tcllib factory commands so the `::ns::cmd` heuristic could be
overridden) was attempted in `0cac3f63`.  Result: covers
`struct::set` cleanly (~17 firings) but does **not** apply to the
dominant case — the variable-as-cmd FPs aren't about an *external*
factory's return type, they're about a *local literal assignment*
whose value flows through `$var` to a dispatch site.  Registry data
is the wrong lever.

### Closure proposal: per-SSA-version literal-cmd-name tracking

Add a small lattice fact alongside the existing SCCP
`values: dict[(name, version), LatticeValue]`:

```python
class CommandNameLattice(Enum):
    UNKNOWN   # default for variables not assigned a literal name
    KNOWN     # variable holds a specific literal command name
    OVERDEF   # multiple distinct literal names reach this version
```

with `known_command_names: dict[(name, version), str]` populated by
**any of** these SSA-statement shapes:

1. `set x <literal>` where `<literal>` is a known registered command
   or user proc qname.
2. `set x [list <literal> …]` — first list element is a literal cmd
   name; downstream `eval $x` becomes a known dispatch.
3. Param flow: propagate via the same `_collect_call_site_constants`
   mechanism stage-2 added for D3-P2 (the dict-with closure).  When
   *every* caller passes the same literal cmd name for a param, the
   callee's param `(name, 0)` enters `KNOWN`.
4. Phi join: lattice rules same as SCCP — `KNOWN(a) ⊔ KNOWN(b)` is
   OVERDEF when `a != b`, else `KNOWN(a)`.

Consume in `_diag_var_command`: when `$var` is the command word at
a dispatch site and `known_command_names[(var, use_version)]` is
KNOWN, treat the dispatch as if the resolved name were written
literally (existing W307 suppression / W123 firing fall out for
free).

### Acceptance criteria

1. Residual W307 across the 836-file corpus drops by **≥ 1 800**
   (the cases where SCCP already proves an exact `(name, version)`
   has a single literal write reaching it — the easy tier).
2. **Zero new diagnostics** on the corpus (no FP cascade from a
   wrongly-inferred name).
3. The reviewer's specific repro (`[::pkg::plain] arg` — a known
   *external* command with `return_type` set) starts firing W307
   when `::pkg::plain` is registered with `return_type=STRING`.
   This currently relies on the registry data, but once VAR-as-cmd
   tracking exists, the same evidence flows through to the
   user-proc local-literal case too.
4. New regression family `tests/test_fp_obj_var_as_cmd.py` with at
   least 6 TP+TN pairs (literal-then-dispatch silent; mixed-callers
   conservative; phi-join still fires; etc.).

### Files & line anchors (start here)

- `compiler/core_analyses.py:_run_sccp` — extend the lattice walk to
  carry `known_command_names` alongside `values`.
- `compiler/core_analyses.py:_collect_call_site_constants` — already
  collects per-callee literal arg values; extend the predicate to
  also accept "this param is always called with a literal cmd-name
  string" and seed param `(name, 0)` accordingly.
- `analyser/_analyser/_diag_var_command.py:_is_object_returning_command_head`
  — extend to consult `known_command_names` before falling back to
  the SCCP-CONST/`::`-prefix heuristics.
- `compiler/registry/runtime.py` — no change; the registry data
  layer is sufficient once the inference exists.

### Closes (when shipped)

- D3-P5 (🔄 → ✅) — `[::pkg::plain]` external returns string.
- D4-F6 (🔄 → ✅) — object-factory inference for `::ns::` and `new`.
- SF-1 (🔄 → ✅) — the registry-data partial becomes self-closing
  because the dominant 99 % gap moves into the inference layer.

---

## B. TclOO method-body lowering to per-method FunctionUnits

### Problem (verified vs tclsh)

```tcl
oo::class create C {
    method pure_helper {} { expr {1 + 1} }
    method m {} {
        set unused [my pure_helper]      ;# pure RHS — should fold
        puts done
    }
}
```

tclsh: `[my pure_helper]` returns `2` with no side effects.  Stage-2
SF-2 wired the optimiser to *consult* `ctx.interproc.methods` when
deciding whether a `my <method>` cmd-sub RHS is safe to delete (the
`_word_has_observable_side_effect` / `_assignment_safe_to_delete`
gate, see `compiler/optimiser/_elimination.py`).  But
`ctx.interproc.methods` is **always empty today** because TclOO
method bodies aren't lowered to per-method `FunctionUnit`s; they
live inside the enclosing class's `ClassDef.methods` as un-lowered
IR fragments.  So the gate exists but never trips.

### Closure proposal: lower methods to FunctionUnits

1. Extend `compiler/compilation_unit.py:_compile_source_inner` to,
   for each `ClassDef` it discovers, walk `cd.methods.items()` and
   produce a `FunctionUnit(qname=f"{cls}::{method}", ...)` per
   method, **using the same lowering pipeline as `proc`**.  Methods
   already have parsed-arg lists + parsed body tokens in
   `ClassDef.methods` (see `analyser/_analyser/_oo.py`), so the IR
   build can be invoked directly.
2. Populate `InterproceduralAnalysis.methods: dict[(cls, method),
   ProcSummary]` from those FunctionUnits — same `pure=` /
   `effects=` fields the existing `summaries: dict[qname,
   ProcSummary]` carries.
3. The SF-2 gate at `_elimination.py:_method_pure` already consults
   this dict; it just needs the dict to be populated.

### Acceptance criteria

1. `FP-OPT-12` (the stage-2 SF-2 entry) flips from "wiring landed
   but inactive" to "fires when method body is pure" — its current
   xfail-style assertion becomes an active TP.
2. New corpus impact: ≥ **80** O126 deletions in tcllib that today
   are blocked by the conservative `my <method>` impure-default.
   Verified against tclsh runtime equivalence (`make test-opt` +
   `tests/test_optimiser_vm_equivalence.py`).
3. **Zero new diagnostics** on the corpus.
4. New regression: `tests/test_fp_opt.py::test_FP_OPT_12_method_pure_*`
   pair (TP: pure-method RHS deleted; TN: impure-method RHS
   preserved) — both already drafted but inactive; flip the active
   assertion on.
5. Bytecode identity (`tests/test_bytecode_identity.py`, 654 tests)
   stays green — methods are analysis-only artefacts, no codegen
   change.

### Files & line anchors

- `compiler/compilation_unit.py:_compile_source_inner` — add the
  per-method lowering loop after class discovery.
- `compiler/interprocedural.py:ProcSummary` — extend (or alias)
  for method summaries; the existing dataclass should fit.
- `analyser/_analyser/_oo.py:_handle_class_create` / `_handle_snit_type_command`
  — already produce `ClassDef.methods`; the lowering consumes that.
- `compiler/optimiser/_elimination.py:_method_pure` — consumer
  already in place (stage-2 SF-2).

### Closes (when shipped)

- SF-2 (🔄 → ✅) — TclOO method purity for O126.

### Related precision wins (out-of-scope but worth noting)

Once methods exist as `FunctionUnit`s, the **same** machinery
unlocks:
- Per-method shimmer detection (currently methods are walked through
  the class scope's IR which is coarser).
- Per-method dead-store / read-before-set diagnostics with correct
  proc-local scope (today some W210s on method bodies are
  conservatively suppressed because the body is treated as the
  class scope).
- Per-method object-provenance inference (`my method` →
  `return [Foo new]` → method's return is OBJECT).

These wins are listed for visibility only — they're additional
**precision** improvements, not soundness gaps.

---

## How to consume this doc

If you're picking up a PARTIAL:

1. **Confirm the prereq isn't already in flight** — check
   `claude/parser-compiler-algorithms-9FN6G` for active work on
   either A or B.  If yes, coordinate; if no, proceed.
2. **Build the prereq end-to-end** — including the corpus delta
   audit (`bench/diag_dump.py` before/after) and the bytecode-
   identity gate.  Don't ship the consumer without the producer.
3. **Flip the PARTIAL rows** in `review-findings-tracker.md` to
   ✅ FIXED with the closing commit hash + FP.md cross-link.
4. **Activate the inactive tests** drafted for the PARTIAL (search
   for `@pytest.mark.xfail` or "PARTIAL" docstrings in
   `tests/test_fp_*.py`).
5. **Delete the relevant section** from this doc as you close each
   prereq.  When both A and B are closed this file can be deleted
   entirely.

The 4 PARTIALs are the *only* unfinished business from the stage-2
wave — everything else lands fully closed.  The merge of
`claude/stage-2-fp-catalog` is clean modulo this doc.
