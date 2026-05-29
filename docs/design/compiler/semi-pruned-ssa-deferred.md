# Semi-pruned SSA — investigation and deferral

## Goal

Reduce phi nodes by switching `compiler/ssa.py:_phi_vars` from **minimal** SSA
(a phi wherever a variable has ≥2 def sites) to **semi-pruned** SSA (Briggs et
al.): place phis only for *non-local* names — those with an upward-exposed use
in some block (used before being redefined, or read by a branch condition).
On a 250-file real-world corpus this cut placed phis from **8335 → 6683 (~20%)**
(`bench/phase2_ssa.py`).

## Why it is deferred

Semi-pruned SSA is **not output-equivalent** today: it added **+244 diagnostics**
on the corpus (151× `W220` "assignment never read", 93× `W211`), **zero removed**.

Root cause (verified, `compiler/core_analyses.py`):

1. The dead-store analysis treats a definition as live if its version appears
   in any **use set _or any phi-incoming edge_** (see the module docstring and
   `_sccp`'s `used_keys` construction, ~L889-899).
2. Minimal SSA places phis purely from **def sites**, independent of uses. So a
   multiply-defined variable always gets a phi, and that phi's incoming edges
   count as "uses" — suppressing `W220`/`W211` even when the phi is dead.
3. The IR's use-tracking is **incomplete** for some reads. Concrete case:
   `::tcl::idna::punydecode` (`tmp/tcl9.0.3/library/cookiejar/idna.tcl`) reads
   `$w` (the punycode weight) in the loop body, but `_uses` reports **no** read
   of `w` anywhere — the reads sit in forms `_uses`/`_vars_in_word` don't fully
   traverse. Minimal SSA still places `w`'s phi (it has 2 def sites), and that
   **dead phi masks** the otherwise-false `W220 "assignment to w is never read"`.

Removing the dead phi (semi-pruned) exposes the latent gap as a **false
positive**: `w` *is* read in the source, so "never read" is wrong.

## Deeper diagnosis (regression dig)

**Quantified:** of the 244 new diagnostics, **~191 (78%) are false positives**
(the variable *is* read somewhere — the read is just untracked) and ~53 are
candidate true dead stores (proxy over-counts these). So semi-pruned is
dominated by false positives from the use-tracking gap, not an accuracy win.

**Exact root cause — confirmed minimal case:**
```
incr i [expr {$w}]      ->  _uses sees ['i']      ($w MISSED)
set  i [expr {$i+$w}]   ->  _uses sees ['i','w']  (direct expr: OK)
set  t [string length $w] -> _uses sees ['w']     (plain-word cmd-sub: OK)
```
The `$w` in `incr i [expr {$w}]` lives inside **expr braces within a command
substitution**. `_uses` → `_vars_in_word` uses the *shallow* scanner
`_VAR_REF_SCANNER` (`recurse_into_script_roles=False`): it descends the `[...]`
command sub to the `expr` command but does **not** descend `expr`'s braced
EXPR-role argument, so the vars inside are invisible (Tcl doesn't substitute
inside `{...}`; only `expr` does, at eval time). Same EXPR-not-parsed gap as
Phase 0.

**The masking mechanism:** `compiler/core_analyses.py` `_dead_stores`
(~L1270-1276) adds every phi's incoming versions to the `used` set
**unconditionally — even when the phi's result is itself never used (a dead
phi).** Minimal SSA places a phi for every multiply-defined variable from def
sites alone, so the dead phi's incoming edge marks `set w 1` as "used",
suppressing the (otherwise-false) dead-store warning. The over-approximation is
what keeps the dead-store check sound (no FPs) under minimal SSA — at the cost
of missing real dead stores to multiply-assigned variables.

**The fix infrastructure already exists.** `compiler/var_refs.py` has a deep
scanner mode (`recurse_into_script_roles=True`, used by
`_DEEP_VAR_REF_SCANNER`) that *does* descend EXPR-role args:
```
_DEEP_VAR_REF_SCANNER.scan_word('[expr {$digit * $w}]') -> ['digit', 'w']
```

## Concrete remediation (sequence)

1. **Fix the use-tracking gap (root, independently valuable).** Make `_uses`
   extract expr-internal reads from command-substituted exprs — e.g. route
   `IRIncr.amount` and `IRCall` non-body words that contain `[expr {...}]`
   through `_DEEP_VAR_REF_SCANNER` (or a targeted expr parse), instead of the
   shallow scanner. This corrects dead-store, read-before-set, liveness and
   SCCP use-tracking *today*, under minimal SSA. **Blast radius:** `_uses`
   output grows → SSA use sets, SCCP, liveness and several diagnostics shift;
   validate byte-identical-or-tclsh-audited on the corpus, and watch perf (the
   deep scan is heavier — apply selectively to words containing `[expr`).
2. **Then ship semi-pruned/pruned SSA + make `_dead_stores` count phi-incoming
   only for transitively-live phis.** With reads now complete, `w` is
   upward-exposed (phi kept) so the FP is gone, and the ~53 genuine dead stores
   minimal SSA was masking get correctly reported — an accuracy win with no FPs.

Fix 1 is the prerequisite; Fix 2 alone would make *minimal* SSA regress too
(removing the masking before reads are tracked).

## Update: partial Fix 1 shipped (name-level)

The first slice of Fix 1 is implemented and **purely positive**: expr-internal
reads from command-substituted exprs are now recovered **at the name level** and
fed to the dead-store (W220) and set-but-never-used / unused-param (W211/W214)
checks, which can only *suppress* findings. Result on the 250-file corpus:
**56 false positives removed (23 W220, 18 W211, 15 W214), 0 added.**

- `compiler/var_refs.py`: new `recurse_into_expr_roles` option (descends
  `ArgRole.EXPR` only, not BODY).
- `compiler/ssa.py`: `expr_substitution_read_names(words)` =
  EXPR-deep minus shallow scan.
- `compiler/core_analyses.py`: `_expr_substitution_reads(cfg)` folded into
  `_collect_used_names` and used as a name-level skip in `_dead_stores`
  (covers IRCall/IRBarrier/IRIncr/IRAssignValue words + CFGReturn values).

**Deliberately NOT routed into `_uses`/SSA `stmt.uses`**, because exposing these
reads to read-before-set (W210) creates false positives where the matching def
lives in a sibling command substitution (`[set ee ...] ... [expr {$ee}]`) or in
a proc defined by an *unrecognised* definer (e.g. `clay::PROC`, whose params
aren't seeded). Those are two further latent gaps:

- **(C) cmd-substitution assignment defs** (`[set x ...]`) not tracked as defs.
- **(D) unrecognised proc-definers** (`clay::PROC`) → params not seeded, bodies
  not lowered as separate scopes.

The remaining (versioned) Fix 1 + Fix 2 (pruned SSA) require (C) and (D) so that
reads and defs are tracked symmetrically before the dead-phi masking is removed.

## What this means

- The +244 are **not** an accuracy win — they are false positives surfaced by an
  underlying IR use-tracking gap that minimal SSA happened to paper over.
- Shipping semi-pruned safely requires **both**:
  - **(A)** Decouple the dead-store check from dead phis: a phi-incoming edge
    should count as a use only when the phi's result is **transitively live**
    (i.e. full *pruned* SSA semantics), not merely present.
  - **(B)** Close the IR use-extraction gap so reads like `punydecode`'s `$w`
    are captured (otherwise even fully-pruned SSA drops `w`'s phi and `W220`
    still fires falsely).

Either alone is insufficient; (B) is the deeper pre-existing bug.

## Reproduction

```
python bench/phase2_ssa.py --phis tmp/after.json            # phi count (current = minimal)
python bench/phase0_descend.py --capture tmp/diag.json      # full analyser diagnostics
```
With the semi-pruned `_phi_vars` patch applied, diff the two diagnostic captures
by code to see the W220/W211 additions; `idna.tcl:233` (`set w 1`) is the
canonical false positive.

## Status

Deferred. Minimal SSA retained on `main`. Next step is the IR use-tracking fix
(B), then the pruned-SSA dead-store decoupling (A); both validated by
`bench/phase0_descend.py` byte-identical diagnostics + `make test-opt`.

## Update 2: prerequisites done; placement implemented; re-deferred on two surfaced bugs

**Gap B (the deeper pre-existing bug) is fixed** (commit "Phase 2 (prereq):
complete expr-cmdsub use-tracking…"): `_vars_in_word` now uses the EXPR-deep
scanner, so a read hidden in a command-substituted expr (`incr i [expr {$w}]`)
flows into SSA `uses`. The canonical `idna.tcl:233` false dead-store is gone
*under minimal SSA*. Fixing gap B also surfaced — and we fixed — an unrelated
SCCP soundness bug: global/namespace/upvar-aliased/traced variables were
const-folded (`set ::g 5; mut; $::g` wrongly folded to 5; tclsh: the mutated
value). Those names are now forced OVERDEFINED in `_sccp`.

**Semi-pruned placement is implemented and correct** — the Briggs non-local
set is the upward-exposed-use set:

```
nonlocal = { v : some block reads v before (re)defining v in that block }
           (statement uses, plus branch-condition / return reads, checked
            against defs-so-far in the block)
phi sites = iterated dominance frontier of def-sites restricted to nonlocal
```

On a quick check it cut phis ~as expected and **no longer adds the false dead
stores** (gap B closed). But enabling it is **still not output-equivalent**, on
two *new* fronts (both pre-existing latent bugs it merely unmasks):

1. **Phi-merge shimmer on never-read variables.** `set x 1; if {$c} {set x
   hello}` with `x` never read places no phi under semi-pruned, so the S100/
   S101 "intrep changes at merge point" warning disappears. This is *defensibly
   more precise* (an unobserved merge has no shimmer cost), but it is a
   behaviour change to the whole shimmer family and needs a corpus
   net-positive audit + test updates (5 tests in `test_shimmer.py`,
   `test_type_propagation.py`, `test_tcl9_type_inference.py` assert the phi on
   an unread `x`).

2. **Malformed O109 range for dead stores in `switch` arms.** Once the masking
   phis are gone, single-statement switch arms (`one {set y 1}` with `y` never
   read) become genuine dead stores the optimiser now eliminates — but the
   elimination's reported source range is mis-computed (covers `set y 1}\n␣␣`,
   including the arm's closing brace), tripping
   `test_highlight_ranges.py::…[switch]`. This is a standalone range-narrowing
   bug in dead-store-elimination reporting for switch-arm bodies, independent of
   SSA, and should be fixed on its own.

**Remaining work to flip semi-pruned on** (todo): (a) fix the switch-arm O109
elimination range; (b) decide + validate the shimmer-on-unread-var behaviour
(corpus audit that the dropped warnings are all unobserved-value FPs), update
the 5 tests; (c) corpus diagnostic diff confirming only the ~53 genuine dead
stores appear, zero FPs; then gate `_phi_vars` on the non-local set.

## Update 3: gap-B (expr reads into SSA `uses`) reverted — needs scanner scoping

Routing expr-cmdsub reads into `_vars_in_word`/SSA `uses` (Update 2's gap-B fix)
**regressed read-before-set on the corpus (+55 W210)** and was reverted; the
**global const-fold soundness fix was independent and is retained**.

Root cause of the regression: the IR stores a *braced-literal* (data) argument
with its outer braces stripped — `append buffer {if {![file exists $x]} …}`
becomes arg text `if {![file exists $x]} …`. The EXPR-deep scanner then
mis-reads that **data** as a live `if` command and descends its EXPR-role
condition, flagging `$x` read-before-set. (It also amplified the gap-D
unrecognised-proc-definer FPs — `clay::PROC`, `report::defstyle` — whose params
aren't seeded.)

**Correct fix for when gap B is re-attempted:** make `recurse_into_expr_roles`
descend EXPR-role args **only within `[...]` command substitutions**, never at
the top level of a (possibly brace-stripped) word — so `incr i [expr {$w}]`
still recovers `$w` (the expr executes) while `append buffer {if {…$x…}}` does
not (the brace-stripped literal is data). Equivalently, preserve the
braced-literal marker through lowering so data args are scanned as opaque
strings. Until then `_vars_in_word` stays shallow and expr-cmdsub reads remain
name-level/suppress-only via `expr_substitution_read_names` (dead-store/unused
only, never read-before-set). The idna `set w 1` dead-store FP stays fixed via
that name-level path.

## Update 4: ENABLED — both sub-issues resolved

Semi-pruned `_phi_vars` (gate on `_nonlocal_names`) is now **on**.  The two
blockers from Update 2 are resolved:

1. **switch-arm O109 range bug — FIXED** (commit "Fix optimisation range
   including an enclosing close brace…"): `_full_command_range` now tracks brace
   nesting across the command's words, so an enclosing `}` glued to the last
   word (`set y 2}`) no longer inflates the rewrite range.

2. **phi-merge shimmer on never-read vars — confirmed a precision *improvement*,
   not a regression.** A merge whose value is never read costs nothing at run
   time, so the S100/S101 "changes intrep at merge point" warning on it is a
   false positive; semi-pruned omits the (dead) phi and the FP disappears.
   Verified: shimmer is **kept** when the merged variable is read (`… ; puts $x`
   still flags S100/S101) and **dropped** only when it is never read.  The 5
   tests that asserted the old over-firing were updated to read the variable.

The original +244-diagnostic regression (Update 1) does **not** recur: gap B's
name-level read recovery (`expr_substitution_read_names`, fed into
`_dead_stores`/unused) already suppresses the dead-store FPs that the dead-phi
masking used to hide, so the canonical `idna.tcl:233 set w 1` stays clean.
Validated by `make test-py` (14252 passing) + corpus diagnostic diff.
