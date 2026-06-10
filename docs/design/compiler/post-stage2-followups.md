# Post stage-2 architectural follow-ups

This doc tracked the **4 PARTIAL closures** left after the stage-2 review
wave (branch `claude/stage-2-fp-catalog`).  Three are now fully closed
(SF-2 via TclOO method-body lowering; the variable-as-command tier of
SF-1; the precision-firing side of D3-P5 / D4-F6 was already shipped in
stage-1/2).  The only remaining thread is a registry-data residual on
D3-P5 / D4-F6 — see **What remains** at the bottom.  The original
architectural proposals are preserved in git history (this file's
pre-closure revision) if the full lattice/method design is ever needed.

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

## Status snapshot (updated post-merge)

Both prerequisites were picked up after the stage-2 merge.

| Prereq | Blocks | Status |
|---|---|---|
| **A.** VAR-as-cmd type inference | D3-P5, D4-F6, SF-1 | ✅ variable-as-command tier closed (existing SCCP lattice, hardened) · residual is registry-data-bound, not an inference gap |
| **B.** TclOO method-body lowering to per-method `FunctionUnit`s | SF-2 | ✅ FIXED — §B removed below |

**SF-2** is fully closed (see `review-findings-tracker.md` / `FP-OPT-12`);
its §B section has been deleted per the consume-this-doc protocol.

**§A** turned out **not** to need the new `known_command_names` lattice
this doc originally proposed: the dominant 99% variable-as-command tier
(per the SF-1 census) was already covered by the SCCP CONST/CONSTSET
lattice plus the D3-P2 call-site param-constant seeding that landed in
stage-1/2 — a literal command name flows through the *same* constant
lattice as any other string.  See the rewritten §A for what was actually
done and what residual remains.

---

## A. VAR-as-cmd type inference — closed via existing SCCP lattice

### Outcome

The variable-as-command dispatch FPs the reviewer flagged
(`set cmd parse; $cmd "5 + 3"`) **do not fire W307 today** — they were
already suppressed by the shipped SCCP CONST/CONSTSET path in
`_diag_var_command.py`, which checks whether every value a `(name)` can
hold at a dispatch site is a registered command or user proc.  The
D3-P2 closure additionally seeds callee params from call-site literals,
so the param-flow form (`proc run {cmd} {$cmd x}; run parse`) is covered
too.

A separate `known_command_names` lattice (this doc's original proposal)
would have **duplicated** that machinery, so it was deliberately not
built.

### What was added on closure

1. **Per-SSA-version refinement** (`_diag_var_command.py`,
   `_precise_cmd_values`): the suppression now reads the SCCP value at
   the dispatch's *exact use-version* instead of the per-function merged
   set, removing the false positive on a variable reassigned from a
   non-command to a command before the dispatch
   (`set c notacommand; set c parse; $c x`).  Purely additive — it only
   strengthens suppression when the precise value is provably a known
   command, never broadens a fire.
2. **Regression family** `tests/test_fp_obj_var_as_cmd.py` — 9 TP/TN
   pairs: literal-then-dispatch silent; non-command fires; param-flow
   (single caller silent / non-command fires); phi-join (two commands
   silent / command+non-command fires); reassignment precision; mixed
   callers conservative.

### Residual (not closeable by inference)

The **cmd-sub-as-command external** case — `set x [::pkg::plain]; $x op`
where `::pkg::plain` is an *unregistered external* command with no proc
visible in the file — still suppresses W307 (see `FP-OBJ-14`).  This is
**not** a static-inference gap: the value is the return of an unknown
external command, so only registry `return_type` data can resolve it
(SF-1's registry-coverage tier, folded into D1-11).  D3-P5 / D4-F6 stay
🔄 PARTIAL for this residual; the variable-as-command portion is closed.

---

---

## What remains

Only one thread is still open, and it is **not** an inference gap:

- **D3-P5 / D4-F6 residual** — registry `return_type` coverage for
  unregistered external factory commands (`[::pkg::plain]` style
  cmd-sub-as-command).  This is incremental registry-data work tracked
  under SF-1 / D1-11, not an algorithmic prerequisite.  When that data
  lands, flip the D3-P5 / D4-F6 rows in `review-findings-tracker.md` and
  the `FP-OBJ-14` verdict, then this file can be deleted entirely.

SF-2 (§B) and the variable-as-command tier of §A are fully closed with
regression tests; the corresponding tracker rows are ✅ FIXED.
