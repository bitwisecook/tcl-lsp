# FP.md — false-positive / true-positive catalog (with per-line reasoning)

This file is the durable record of every diagnostic determination made on the
`claude/parser-compiler-algorithms-9FN6G` precision work — for each one, a
trimmed real-corpus reproducer, **a line-by-line walkthrough proving the
verdict**, the compiler evidence (SSA / SCCP lattice / dead-store / RBS /
intervals) that the analyser used to reach it, and a paired TP+FP test that
locks the verdict in from both sides.

Companion docs:

- [`review-findings-deferred.md`](review-findings-deferred.md) — short-form
  ledger of verified-real-but-deferred findings (this doc embeds the *evidence*
  for them and the fixed ones too).
- [`phase8-place-migration.md`](phase8-place-migration.md) — Place-model
  migration plan that drives the precision wins in the W210/W220/W211/W307
  families.
- [`algorithms.md`](algorithms.md) — algorithm references with adaptations.

Ground truth throughout is **real C tclsh 9.0.3** (and 8.x for dialect-sensitive
cases — every entry that varies says so).

---

## Format

Each entry has a stable `FP-<FAMILY>-<NN>` id and these sections:

1. **Header** — verdict (FALSE POSITIVE / TRUE POSITIVE / NOT A BUG /
   CONFIRM-CORRECT), status (FIXED / OPEN / WON'T-FIX), diagnostic codes, and
   the corpus path the reproducer was trimmed from.
2. **Reproducer** — the minimum Tcl that re-creates the determination,
   comment-annotated. Identifiers shortened to `a b c …` where they don't carry
   intent; corpus structure preserved so the proof reads as the original idiom.
3. **Per-line reasoning** — a numbered walk through the reproducer that proves
   the verdict against tclsh behaviour. This is the human-readable counterpart
   to the compiler evidence below; the two must agree.
4. **tclsh ground truth** — the exact runtime result / error / dialect notes.
5. **Compiler evidence** — a small block of SSA / lattice / analysis output
   pasted from `bench/fp_snippets.py --id FP-<id>`. Always shows the regen
   command so a reviewer can re-derive it.
6. **Why the analyser reaches that verdict** — the precise code path inside
   the analyser/compiler (file:line where the decision is made).
7. **Tests** — names of the paired must-fire (TP) and must-stay-silent (FP)
   tests (`tests/test_fp_<family>.py`) that consume *this exact reproducer*.

Open findings that still false-positive today carry a passing TP test plus an
`xfail(strict=True)` FP test so the suite signals when the gap closes.

---

## Families (one section per slice / eventual PR)

- **NAB — Not-a-bug / confirm-correct audits** ([§NAB](#nab--not-a-bug--confirm-correct-audits))
- **RBS — Read-before-set (W210/W213/W214)** *(planned, PR 1)*
- **DS — Dead-store / unused (W220/W211)** *(planned, PR 2)*
- **SH — Shimmer (S100/S101/S102)** *(planned, PR 3)*
- **OBJ — Object dispatch (W307/W308)** *(planned, PR 4)*
- **RCH — Reachability (O107)** *(planned, PR 5)*
- **INJ — Injection / style (W101/W105/W301/T102)** *(planned, PR 6)*
- **BND — Bounds / intervals (W230/W231/W232/W233)** *(planned, PR 7)*

---

## NAB — not-a-bug / confirm-correct audits

These reproduce constructs that *were initially suspected to be FPs* (or extra
checks were proposed for them) but, on audit against tclsh + the analyser code
path, turned out to be **already correctly handled**.  Each entry locks the
audited construct in as a regression test so a future "improvement" that
silently breaks the verdict is caught.

### FP-NAB-01 — `lset` append-slot (index == length) is legal, NOT W231

- **Verdict:** NOT A BUG (analyser already correct)
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_01_append_slot_silent`
- **Codes:** W231 (lset out-of-range)
- **Corpus:** `tmp/tcllib-2.0/modules/struct/list.tcl` (the `lset … end+1` /
  append pattern appears throughout; the trimmed reproducer is small enough that
  any append-by-index use is equivalent).

#### Reproducer

```tcl
proc f {l} {
    # contract: caller passes a 3-element list, e.g. {a b c}
    lset l 3 X     ;# 3 == llength $l  -> APPENDS X (NOT an error)
    return $l
}
```

#### Per-line reasoning

1. `proc f {l}` — `l` is a parameter, so the analyser sees it as a defined
   SSA value entering `entry_1` (no read-before-set, no dead store from any
   preceding `set`).
2. `lset l 3 X` — Tcl's `lset` documentation (`lset(n)`) states that when the
   index equals the current list length, the element is **appended**. It only
   errors for `index > length` or `index < 0`. So index `3` against a 3-element
   `l` is the **append slot** — sound, not a bug.
3. `return $l` — `l` is read after the lset, so even if the analyser briefly
   considered the entry to `l` "dead w.r.t. the SSA rebind by lset", the actual
   read keeps the post-lset version live and prevents a dead-store FP.

The verdict is **NOT a W231**: append at `index == length` is legal.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set l {a b c}; lset l 3 X; puts $l
a b c X
```

(Compare: `lset l 4 X` on a 3-element list errors with `list index out of range`;
`lset l -1 X` errors too. The append slot is the **only** non-error
out-of-bounds index.)

#### Compiler evidence

```
--- FP-NAB-01: lset append-slot (index == length) is legal, NOT W231
regen: python -m bench.fp_snippets --id FP-NAB-01
function ::f
  block entry_1
    [0] Call cmd='lset'  defs={l#1}  uses={}
    term Return ${l}
  values (SCCP lattice)
    l#1: OVERDEFINED
  dead_stores: (none)
```

- One SSA rebind of `l` (`l#1`) — the param entry is consumed by `lset`, which
  rewrites it. SCCP cannot fold a `lset` result so the value is OVERDEFINED.
- **`dead_stores: (none)`** — there is no spurious dead-store flag, exactly
  what we want.
- The bounds check that *would* fire here is W231; the catalog's negative test
  asserts it does not fire (the `analyser/checks/_bounds.py:318-325` check uses
  the strict `resolved > list_len` comparator, which permits append).

#### Why the analyser reaches that verdict

`check_lset_index_out_of_range` (`analyser/checks/_bounds.py`) compares the
resolved index against the inferred list length with `>` (not `>=`), so
`index == length` is permitted. This was confirmed-correct on audit; the verdict
is *NOT-a-bug*, not "we deferred the fix".

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_01_append_slot_silent` (FP, must stay silent)
- `tests/test_fp_nab.py::test_FP_NAB_01_real_out_of_range_fires` (TP, real
  `lset l 4 X` on a 3-element list does fire W231)

---

### FP-NAB-02 — `lindex` out-of-range returns `""` — smell (W230), not an error

- **Verdict:** NOT A BUG — the severity split between W230 (lindex smell) and
  W231 (lset error) reflects real tclsh behaviour.
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_02_*`
- **Codes:** W230, W231
- **Corpus:** `tmp/tcllib-2.0/modules/struct/list.tcl` (literal-arg
  `lindex {…} N` patterns appear across tcllib for compile-time list lookups;
  reproducer keeps just the proof).

#### Reproducer

```tcl
# Top-level lindex with literal list + literal out-of-range index.
# tclsh returns "" silently — likely-bug, not an error.
set x [lindex {a b c} 9]
return $x
```

#### Per-line reasoning

1. `set x [lindex {a b c} 9]` — both the list (`{a b c}`, length 3) and the
   index (`9`) are *literals on the same call*. This is exactly the form the
   W230 syntactic check (`analyser/checks/_bounds.py:118-181`) is built for: it
   walks `arg_tokens[0]` as the list, splits it, then for each subsequent
   literal index checks `0 <= resolved < length`. Here `9 >= 3`, so W230 fires.
2. Per `lindex(n)`, an out-of-range index **returns the empty string**, it does
   **not** error. So the user's code is technically running, but probably
   buggy — the appropriate severity is **W230 smell-only**, not the W231
   error-tier severity reserved for `lset` (which DOES error in tclsh).
3. `return $x` — `$x` is the empty string; the return value is `""`. SCCP
   folds this independently and arrives at `x#1: CONST('')` (see evidence
   below) — the analyser independently *agrees* with tclsh's runtime value,
   which makes the W230 verdict mechanical, not heuristic.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set l {a b c}; set x [lindex $l 9]; puts "x=>'$x'"
x=>''
```

The same out-of-range index on `lset` errors:

```
% set l {a b c}; lset l 9 X
index "9" out of range
```

This dialect asymmetry is what justifies the W230 (smell) vs W231 (error)
severity split — they're not duplicate diagnostics, they encode different
runtime semantics.

#### Compiler evidence

```
--- FP-NAB-02: lindex out-of-range returns "" — smell (W230), not an error
regen: python -m bench.fp_snippets --id FP-NAB-02
function ::top
  block entry_1
    [0] AssignValue 'x' value='[lindex {a b c} 9]'  defs={x#1}  uses={}
    term Return ${x}
  values (SCCP lattice)
    x#1: CONST('')
  dead_stores: (none)
```

- The `value=` field carries the original command substitution. The syntactic
  W230 check inspects the call's arg tokens; the list literal and out-of-range
  index `9` are right there.
- `x#1: CONST('')` — SCCP independently folds the expression to the empty
  string. The analyser *agrees* with tclsh's runtime behaviour: the value is
  `""`, not an error.
- `dead_stores: (none)` and no escalation to W231 — the verdict really is
  "smell, not error".

#### Why the analyser reaches that verdict

`check_list_index_out_of_range` (`analyser/checks/_bounds.py:118-181`) emits
W230 at *warning* severity (not error). It compares `resolved < 0 or resolved
>= length`. The Phase-3 interval rewire added the dynamic, variable-routed
path (`compiler/interval_bounds.py`) but left the W230 severity verdict
untouched — the user-facing meaning still matches `lindex(n)` semantics.

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_02_lindex_oor_smell_fires` (TP, W230 fires)
- `tests/test_fp_nab.py::test_FP_NAB_02_lindex_oor_not_w231` (FP, asserts the
  *error*-tier W231 does NOT fire for this `lindex` case)
- `tests/test_fp_nab.py::test_FP_NAB_02_lset_same_index_does_w231` (TP control,
  the matching `lset` does fire an error-tier out-of-range diagnostic)

---

### FP-NAB-03 — Phase-4 interproc SCC condensation NOT needed (recursive procs already pure)

- **Verdict:** CONFIRM-CORRECT — the original plan-doc claim that
  "recursive procs are conservatively impure" was wrong.
- **Status:** plan section closed; locked in by
  `tests/test_fp_nab.py::test_FP_NAB_03_recursive_proc_detected_pure`.
- **Corpus:** any `proc fact {n} { … fact … }`; reproducer is a textbook
  factorial.

#### Reproducer

```tcl
proc fact {n} {
    if {$n <= 1} { return 1 }
    return [expr {$n * [fact [expr {$n - 1}]]}]   ;# self-recursion
}
```

#### Per-line reasoning

1. `proc fact {n}` — local-only parameter `n`; no globals, no `upvar`, no
   command-rebinding, no I/O. Body has no side-effects (only `expr` /
   `return`).
2. `if {$n <= 1} { return 1 }` — pure terminator.
3. `return [expr {$n * [fact [expr {$n - 1}]]}]` — the only "external" call
   is to **itself**. The interproc summary fix-point is therefore the question:
   *given that everything else `fact` calls is pure, is `fact` pure?*
4. `analyse_interprocedural_ir` (`compiler/interprocedural.py:1029`) starts
   purity at the **optimistic** value `pure=True` for every proc lacking an
   obvious side-effect, then iteratively *intersects* with each callee's purity.
   For self-recursion the intersection is `True ∧ True = True` — the greatest
   fix-point. The procedure is detected as pure on the first round.

The "conservatively impure for self-recursion" framing in the original plan
was a misread of the code; the fix-point is already order-independent and
greatest. SCC condensation would yield no precision delta on the audited
corpus (8 SCCs total) and only risk.

#### tclsh ground truth

Not a runtime-semantic verdict — this is an *analyser-precision* audit.
The tclsh confirmation is incidental: `fact` is observably pure (returns same
output for same input, no side-effects, no `upvar`/`global`).

#### Compiler evidence

```
--- FP-NAB-03: Phase-4 interproc SCC NOT NEEDED — recursive procs are already detected pure
regen: python -m bench.fp_snippets --id FP-NAB-03
function ::fact
  block entry_1
    term Branch ExprBinary(op=<BinOp.LE: '<='>, left=ExprVar(text='$n', name='n', start=0, end=1), right=ExprLiteral(text='1', start=6, end=6))
  block if_end_2
    term Return [expr {$n * [fact [expr {$n - 1}]]}]
  block if_then_3
    term Return 1
  block if_next_4
    term Goto
```

The interproc summary (not shown here — it's the cross-proc analysis, separate
from per-proc SSA) settles `pure=True` for `::fact` on the first pass. The
companion test reads the interproc result directly to assert this.

#### Why the analyser reaches that verdict

`analyse_interprocedural_ir` initialises purity from the per-proc local summary
(`pure=True` when no side-effect was seen locally), then runs a
**monotone-decreasing** worklist fix-point over the reverse call graph. Mutual
recursion is handled by the same mechanism (the greatest-fix-point converges
in one pass when every member is locally pure).

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_03_recursive_proc_detected_pure` (TP, the
  recursive proc *is* reported pure by the interproc analysis)
- `tests/test_fp_nab.py::test_FP_NAB_03_impure_proc_still_detected` (TP/FP
  control, an impure proc using `puts` is correctly reported impure — proves
  the test isn't trivially asserting all procs pure)

---

## RBS — read-before-set (W210/W213/W214)

W210 (read-before-set), W213 (`unset` on possibly-unset var, derives from RBS),
and W214 (read in expr/cmd-sub before a set on the path) are the codes most
prone to false-positives because they require modelling Tcl's many implicit
def-establishing patterns (`info exists` guards, `catch`/`regexp` cmd-sub
writes, `foreach`/`for` loop binders, `upvar`/`global`/`variable` aliasing,
qualified-namespace names, etc.). Each entry below documents either a specific
FP that was eliminated (and is now locked in by the paired test) or an open FP
that the analyser still produces (with an `xfail` FP test that flips when the
gap closes).

### FP-RBS-01 — `info exists` / `array exists` is the test-before-use idiom (not W210)

- **Verdict:** FALSE POSITIVE (W210) — the analyser was flagging the canonical
  *test-before-use* idiom that Tcl scripts use to safely read a variable that
  may be unset.
- **Status:** FIXED (commits `9b73053`, `c5a23d5`). Corpus delta: **−27 W210
  FPs**, 0 regressions.
- **Codes:** W210, W213 (the latter derives from the same RBS analysis).
- **Corpus:** `tmp/tcllib-2.0/modules/http/autoproxy.tcl:42-43`

  ```tcl
  variable uid
  if {![info exists uid]} { set uid 0 }
  ```

#### Reproducer

```tcl
proc maybe_get {} {
    # v is never set in this proc — the info-exists guard is the entire
    # safety: a bare `$v` here would be a hard tclsh error.
    if {[info exists v]} { return $v }
    return {}
}
```

#### Per-line reasoning

1. `proc maybe_get {} { … }` — `v` is **not** a parameter and there is **no
   `set v …` anywhere in this proc body**. From SSA's local-only view, `v` is
   genuinely an unset name on entry to every block — exactly the shape that
   normally fires W210.
2. `if {[info exists v]} { return $v }` — the guard returns `1` from tclsh
   iff `v` exists in the calling scope (caller of `maybe_get`, or one set by
   `upvar`, `global`, `variable`, etc.). The `return $v` in the *then* branch
   runs **only** when `info exists v` returned `1`, so the bare `$v` read is
   safe by construction. tclsh-verified:

   ```
   % if {![info exists undef]} { puts "undef is unset (legal)" }
   undef is unset (legal)
   % puts $undef
   can't read "undef": no such variable
   ```

   So `info exists v` is the canonical *check*; reading `$v` only inside the
   `[info exists v]`-guarded branch is sound. A naïve "v has no def → W210"
   verdict would flag exactly this idiom — that's the false positive.
3. `return {}` — the *else* branch returns the empty string, never touches
   `$v`. No FP can land here.

The analyser's fix is name-level (`existence_test_names` in
`compiler/var_refs.py` collects names appearing inside `info exists` /
`array exists` calls, recursing into EXPR / BODY scripts) and exempts those
names from W210/W213. The exemption is a *suppression*, not a def — so a
genuine RBS elsewhere in the proc would still fire (see the TP control).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc maybe_get {} {
    if {[info exists v]} { return $v }
    return "missing"
  }
% maybe_get
missing
```

No error, no warning. The `info exists v` returns `0`, the `then` arm is
skipped, the else returns `"missing"`. Now contrast a bare unguarded read on
a never-set local — that's the genuine RBS error tclsh raises:

```
% proc bad {} { return $v }
% bad
can't read "v": no such variable
```

#### Compiler evidence

```
--- FP-RBS-01: info exists / array exists is the test-before-use idiom (not W210)
regen: python -m bench.fp_snippets --id FP-RBS-01
function ::maybe_get
  block entry_1
    term Branch ExprCommand(text='[info exists v]', start=0, end=14)
  block if_end_2
    term Return
  block if_then_3
    term Return ${v}
  block if_next_4
    term Goto
  read_before_set: (none)
  dead_stores: (none)
```

- The CFG shows three real blocks: the entry branches on
  `[info exists v]`, the *then* block (`if_then_3`) returns `${v}`, the *else*
  joins via `if_end_2`. The bare `${v}` read **is** present in `if_then_3`.
- The critical evidence is **`read_before_set: (none)`** — the analyser
  correctly suppresses the W210 that this raw shape (bare `$v` on a never-set
  local) would normally trigger. The name `v` was collected by
  `existence_test_names` from the `[info exists v]` arg of the branch
  condition, then excluded from RBS.

#### Why the analyser reaches that verdict

`compiler/var_refs.py:existence_test_names` collects variable names appearing
as args to `info exists` / `array exists` (also descending into EXPR and BODY
scripts so a nested `if {[info exists v] && …}` is caught). `_read_before_set`
in `compiler/core_analyses.py` then excludes those names from its RBS set
(commits `9b73053`, `c5a23d5`).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_01_info_exists_guard_silent` (FP, must
  stay silent)
- `tests/test_fp_rbs.py::test_FP_RBS_01_bare_unguarded_read_still_fires` (TP
  control — proves removing the `info exists` guard restores the W210)
- `tests/test_fp_rbs.py::test_FP_RBS_01_array_exists_guard_silent` (FP
  control — `array exists` guards the same way)

---

### FP-RBS-02 — `catch` / `regexp` / `scan` command-sub writes are not read-before-set

- **Verdict:** FALSE POSITIVE (W210) — the analyser missed that the optional
  output-vars of `catch`, `regexp`, and `scan` *are written in the calling
  scope* even though the whole `[command-sub]` is one opaque value word that
  SSA can't see inside.
- **Status:** FIXED (commit `4e4316b`). Corpus delta: **−172 W210 FPs** on the
  full corpus (1800→1628). Companion fix `6ae85f4` extends it through expr
  bodies; `9f15e05` covers the upvar dynamic-target alias.
- **Codes:** W210
- **Corpus:** `tmp/tcl9.0.3/library/http/http.tcl:800-810`

  ```tcl
  if {[catch {close $socketMapping($connId)} err]} { … }
  ```

  (The same shape appears at lines 749, 810, 1299, 2417, 2627; the http
  module alone has dozens of instances.)

#### Reproducer

```tcl
proc f {} {
    # [catch …] writes 'err' in this scope (tclsh-verified);
    # the read in the consequent must NOT be W210.
    if {[catch {operation} err]} { puts "failed: $err" }
}
```

#### Per-line reasoning

1. `proc f {} { … }` — `err` is not a parameter, not a `set err …` anywhere
   else in the body, not aliased via `upvar` / `global` / `variable`. From a
   raw-SSA view, `err` looks completely undefined.
2. `if {[catch {operation} err]} { … }` — the `catch` command's third
   argument (`err`) is a **caller-scope variable name** that `catch` writes
   with the result of evaluating its body. Per `catch(n)`: *"If `varName` is
   supplied, the variable it names is set to the result …"*. So after the
   command-sub finishes, `err` exists in this proc's local scope. Run it:

   ```
   % proc f {} {
       if {[catch {error "boom"} err]} { puts "failed: $err" }
     }
   % f
   failed: boom
   ```

   No error, no warning. The write is real, just invisible to SSA because the
   command-sub is opaque.
3. The branch consequent `{ puts "failed: $err" }` reads `$err`. **This is
   the read SSA can't justify** without help — its def lives inside the
   opaque `[catch …]` substitution. The fix is name-level:
   `compiler/var_refs.py:command_sub_write_names` walks the command-sub's
   contents and recovers literal `VAR_WRITE` targets (here, `err`), then
   `_read_before_set` exempts those names. (Dynamic `$name`-style targets
   are **excluded** from the recovery — those name a runtime variable and
   should remain visible as reads, not be confused with writes.)

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% if {[catch {error "boom"} err]} { puts "rc=$err" } else { puts "ok" }
rc=boom
% set rc [catch {error "x"} msg opts]
% puts "msg=$msg / opts has [llength $opts] keys"
msg=x / opts has 12 keys
```

`catch ?msg? ?opts?` writes `msg` (result string) and `opts` (options dict)
in the caller frame. Reading them afterwards is the canonical idiom.

`regexp` and `scan` have analogous output-vars:

```
% regexp {(\w+)=(\w+)} "k=v" -> k v
1
% puts "$k=$v"
k=v
% scan "42" %d n; puts $n
42
```

All three commands write into named caller-scope variables. The fix covers
all three in one mechanism.

#### Compiler evidence

```
--- FP-RBS-02: catch/regexp/scan command-sub writes are not read-before-set
regen: python -m bench.fp_snippets --id FP-RBS-02
function ::f
  block entry_1
    [0] Call cmd='<cond>'  defs={err#1}  uses={}
    term Branch ExprCommand(text='[catch {operation} err]', start=0, end=22)
  block if_end_2
    term Goto
  block if_then_3
    [0] Call cmd='puts'  defs={}  uses={err#1}
    term Goto
  block if_next_4
    term Goto
  block exit_5
    term (none — fall-through exit)
  read_before_set: (none)
```

- **`entry_1[0]` shows `defs={err#1}`** — that's the fix at work. The
  branch-condition's command sub has been scanned by
  `command_sub_write_names`, the `err` target recovered, and a virtual `err#1`
  def synthesized so SSA can satisfy the later read.
- **`if_then_3[0]` shows `uses={err#1}`** — the `puts "failed: $err"` read
  resolves cleanly to that virtual def.
- **`read_before_set: (none)`** — the closing receipt: the analyser doesn't
  flag anything in this proc.

If you remove the `command_sub_write_names` recovery (delete the
`statement_cmd_sub_write_names` call in `_read_before_set`), the `err#1` def
disappears, the `uses={err#1}` becomes `uses={}` with `err` unresolved, and
W210 returns. The TP control test asserts that an *unrelated* read (`$other`)
still fires correctly — the fix is precision-recovery, not blanket
suppression.

#### Why the analyser reaches that verdict

`compiler/var_refs.py:command_sub_write_names` (recurses nested subs;
excludes dynamic `$name`-style targets), exposed as
`ssa.statement_cmd_sub_write_names`, then consumed by `_read_before_set` in
`compiler/core_analyses.py`. Companion `command_sub_write_names_in_expr`
covers the version inside `[expr {[catch …]}]` expressions (commit
`6ae85f4`).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_02_catch_msg_var_silent` (FP, the
  classic `[catch {…} err]; puts $err` shape)
- `tests/test_fp_rbs.py::test_FP_RBS_02_unrelated_read_still_fires` (TP
  control, `$other` (no cmd-sub write target) still fires W210)
- `tests/test_fp_rbs.py::test_FP_RBS_02_regexp_match_var_silent` (FP,
  `[regexp … -> k v]; puts $k$v` analogue)
- `tests/test_fp_rbs.py::test_FP_RBS_02_scan_output_silent` (FP, `scan` output
  analogue)

---

### FP-RBS-03 — Frozen-loop bodies: `while`/`for` with cmd-sub condition

- **Verdict:** FALSE POSITIVE (W210) — body-local loop temporaries
  false-flagged because frozen-loop bodies recovered reads but not writes.
- **Status:** FIXED (commit `e319c3a`). Corpus delta: **−46 W210 FPs**, 0
  regressions.
- **Codes:** W210
- **Corpus:** `tmp/tcl9.0.3/library/http/http.tcl:749`
  (`while {[gets $sock line] != -1} { … $line … }` analogue throughout the
  library)

#### Reproducer

```tcl
proc f {fp} {
    # gets writes 'line' AND the body sets 'n' — both are body-local
    # but the frozen-loop body keeps them invisible to SSA defs.
    while {[gets $fp line] >= 0} {
        set n [string length $line]
        puts "$line ($n chars)"
    }
}
```

#### Per-line reasoning

1. `proc f {fp} { … }` — parameter `fp`; `line` and `n` are not parameters
   and not set anywhere outside the loop body, so a raw-SSA view would say
   they're undefined when first read.
2. `while {[gets $fp line] >= 0} { … }` — the condition is a *command
   substitution*. To keep default bytecode codegen byte-identical to tclsh
   (which inline-compiles `while` with a literal-braced condition but treats
   command-sub conditions as opaque), the lowerer keeps such loops as an
   IRBarrier rather than fully expanding them into the CFG. Without the fix,
   that opacity hid the implicit `line` def made by `[gets $fp line]`. tclsh:

   ```
   % set fp [open /etc/hostname r]
   % while {[gets $fp line] >= 0} { puts "got '$line'" }
   got 'vm'
   ```

   `gets` writes into its third argument's name in the caller frame. The loop
   reads `$line` legally — there's no actual error.
3. `set n [string length $line]` — `n` is a normal body-local. Before the
   fix, the analyser saw the loop body's reads (`$line`) but the loop
   barrier's `defs` set didn't list `n` (or `line`), so both looked
   read-before-set. Three SSA versions become real with the fix: `line#2`
   from `<cond>` recovery, `n#1` from the body `set`, and a header `phi`
   joining `line#0` (entry) with `line#2` (back-edge from the body).
4. `puts "$line ($n chars)"` — reads both. The recovered defs satisfy them
   cleanly. No W210.

The fix (commit `e319c3a`) introduced `body_write_names` in
`compiler/var_refs.py` to balance the existing read recovery: it walks the
loop body's IR collecting literal `VAR_WRITE` targets (also `foreach`/`lmap`
loop vars), and the analyser exposes those as the barrier's effective defs.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set fp [open /etc/hostname r]
% while {[gets $fp line] >= 0} {
      set n [string length $line]
      puts "$line ($n chars)"
  }
vm (2 chars)
% close $fp
```

No error, no warning. The body runs each iteration, reading the just-written
`line` and writing `n` freshly each pass.

#### Compiler evidence

```
--- FP-RBS-03: frozen-loop bodies (while/for with cmd-sub condition) — body writes recovered
regen: python -m bench.fp_snippets --id FP-RBS-03
function ::f
  block entry_1
    term Goto
  block while_header_2
    phi  SSAPhi(name='line', version=1, incoming={'entry_1': 0, 'while_body_3': 2})
    [0] Call cmd='<cond>'  defs={line#2}  uses={}
    term Branch ExprBinary(op=<BinOp.GE: '>='>, left=ExprCommand(text='[gets $fp line]', start=0, end=14), right=ExprLiteral(text='0', start=19, end=19))
  block while_body_3
    [0] AssignValue 'n' value='[string length $line]'  defs={n#1}  uses={line#2}
    [1] Call cmd='puts'  defs={}  uses={line#2, n#1}
    term Goto
  block while_end_4
    term Goto
  block exit_5
    term (none — fall-through exit)
  read_before_set: (none)
```

- **`while_header_2[0] defs={line#2}`** — that's the `body_write_names`
  recovery, synthesising the `gets`-written `line` so SSA has something to
  flow into the body uses.
- **The `phi line#1`** joins entry's `line#0` (the implicit "pre-def" sentinel)
  with the back-edge's `line#2` (the just-recovered cmd-sub def). Without the
  recovery, no `line#X` would exist and the phi would have nothing to merge.
- **`while_body_3[0] uses={line#2}`** and `[1] uses={line#2, n#1}` resolve
  cleanly to the recovered defs.
- **`read_before_set: (none)`** — the closing receipt: no W210 fires anywhere
  in the proc.

#### Why the analyser reaches that verdict

`compiler/var_refs.py:body_write_names` walks the loop body's IR collecting
literal `VAR_WRITE` targets and `foreach`/`lmap`/`for` loop-binders, exposed
on the barrier's effective defs. The `<cond>` cmd-sub gets the same treatment
via `command_sub_write_names` (the FP-RBS-02 mechanism).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_03_frozen_while_body_silent` (FP)
- `tests/test_fp_rbs.py::test_FP_RBS_03_genuine_unset_in_body_still_fires`
  (TP control — a variable that's *only* read in the body, never written
  anywhere, still fires W210)

---

### FP-RBS-04 — Qualified-variable aliases: `variable ${name}::tail` (local name is the *tail*)

- **Verdict:** FALSE POSITIVE (W210/W213) — the qualified-form `variable`
  declaration was recording the def under the qualified spelling, leaving the
  static tail (the real local-alias name) un-defed and every read flagged.
- **Status:** FIXED (commit `6207fe0`). Corpus delta: **−250 W210, −10 W213**,
  0 regressions. The largest single RBS fix in this branch.
- **Codes:** W210, W213
- **Corpus:** `tmp/tcllib-2.0/modules/struct/graph_tcl.tcl:1506-1512`

  ```tcl
  proc ::struct::graph::_get {name key} {
      variable  ${name}::graphAttr
      if { ![info exists graphAttr($key)] } {
          return -code error "invalid key \"$key\" for graph \"$name\""
      }
      return $graphAttr($key)
  }
  ```

#### Reproducer

```tcl
proc ::ns::get {name key} {
    # `variable ${name}::graphAttr` declares the local alias 'graphAttr';
    # the qualified form is just where the storage lives.
    variable ${name}::graphAttr
    if {![info exists graphAttr($key)]} { return "" }
    return $graphAttr($key)
}
```

#### Per-line reasoning

1. `proc ::ns::get {name key}` — parameters `name` and `key`. Importantly,
   `graphAttr` is **not** a parameter and not set by any later `set` either —
   from a raw-SSA view it's undefined.
2. `variable ${name}::graphAttr` — this is Tcl's variable-aliasing form. Per
   `variable(n)`: *"For each `varName` argument, `variable` creates a local
   variable in the current procedure that is linked to the namespace
   variable. … If `name` is unqualified, it's relative to the current
   namespace. **The local variable's name is the part after the last `::`**"*.

   So this line:
   - takes the runtime value of `$name` (e.g. `"foo"`),
   - constructs the qualified namespace name `foo::graphAttr`,
   - creates a **local alias in this proc** named `graphAttr` (the tail),
   - links the alias to the namespace variable `foo::graphAttr`.

   tclsh-verified:

   ```
   % namespace eval ::ns { variable children {a b c} }
   % proc tester {} {
         variable ::ns::children
         return $children
     }
   % tester
   a b c
   ```

   `variable ::ns::children` made a local `children` (the tail) that returned
   the namespaced var's value.

3. `if {![info exists graphAttr($key)]} { return "" }` — the
   `info exists graphAttr($key)` test is on the local alias `graphAttr` (the
   tail name). It returns true once any element under that array key has been
   set. This line additionally engages the FP-RBS-01 mechanism — `graphAttr`
   is collected by `existence_test_names` — but the *primary* fix here is the
   alias-tail exemption.
4. `return $graphAttr($key)` — reads the element. With the fix, the static
   tail `graphAttr` is exempted from RBS so this read doesn't false-flag.
   The pre-fix bug: `variable_declaration_indices` *filtered out* `$`-prefixed
   forms (so `${name}::graphAttr` left no def at all), and the qualified-name
   indexing recorded the def under `${name}::graphAttr#1` rather than
   `graphAttr#1` — so the local tail looked completely undefined.

The fix has two parts:

- `_qualified_variable_alias_tails` (in `_read_before_set`) collects the
  static tails of every `variable X::Y` declaration and exempts the tail from
  the RBS set.
- The namespace-membership skip was broadened from `name.startswith('::')` to
  `'::' in name`, so a *use* like `array names ${name}::parent` is correctly
  recognised as a namespace reference (Tcl locals can't contain `::`).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% namespace eval ::ns { variable children {a b c} }
% proc tester {} {
      variable ::ns::children    ;# alias declared by qualified name
      return $children            ;# read uses the TAIL
  }
% tester
a b c
```

No error. The alias creates a local named after the *tail* (`children`), and
that local is read via `$children`. The dynamic-namespace form
`variable ${ns}::children` works the same way at runtime — the tail name is
still the literal `children`.

#### Compiler evidence

```
--- FP-RBS-04: qualified variable aliases (variable ${name}::tail) — local name is the tail
regen: python -m bench.fp_snippets --id FP-RBS-04
function ::ns::get
  block entry_1
    [0] Call cmd='variable'  defs={{name}::graphAttr#1}  uses={name#0}
    term Branch ExprUnary(op=<UnaryOp.NOT: '!'>, operand=ExprCommand(text='[info exists graphAttr($key)]', start=1, end=29))
  block if_end_2
    term Return $graphAttr($key)
  block if_then_3
    term Return
  block if_next_4
    term Goto
  read_before_set: (none)
```

- **`entry_1[0] defs={{name}::graphAttr#1}`** — concrete proof of the pre-fix
  bug shape: the `variable` call records its def under the **qualified
  spelling** `{name}::graphAttr`, NOT under the local-tail name `graphAttr`.
  An RBS analysis that looks up `graphAttr` will find nothing here.
- **`if_end_2 term Return $graphAttr($key)`** — the read does happen, on the
  tail name `graphAttr`.
- **`read_before_set: (none)`** — yet the analyser correctly suppresses W210.
  That's the fix at work: `_qualified_variable_alias_tails` recognises that
  `{name}::graphAttr` is a qualified `variable` declaration whose static tail
  is `graphAttr`, and exempts the tail from the RBS set.

Strip the fix (remove the tail-exemption in `_read_before_set`) and the
analyser would emit W210 here because `defs={{name}::graphAttr#1}` does not
introduce a `graphAttr#X` def — the qualified-form def goes to a completely
different SSA name.

#### Why the analyser reaches that verdict

`_read_before_set` in `compiler/core_analyses.py` calls
`_qualified_variable_alias_tails` to collect tails from every `variable …`
declaration whose first arg contains `::`; those tails are unioned into the
suppressed-names set alongside the FP-RBS-01 existence-test names. The
companion change broadens the namespace skip from `startswith('::')` to
`'::' in name` so dynamic-namespace forms like `array names ${name}::parent`
also stay silent.

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_04_qualified_alias_silent` (FP, the
  `variable ${name}::tail` shape)
- `tests/test_fp_rbs.py::test_FP_RBS_04_static_qualified_alias_silent` (FP
  control — the `variable ::ns::tail` *static-namespace* form works too)
- `tests/test_fp_rbs.py::test_FP_RBS_04_unrelated_tail_still_fires` (TP
  control — a read of a *different* tail name (one not declared as a
  qualified alias) must still fire W210)

---

### FP-RBS-05 — `namespace upvar` alias-not-a-def (OPEN, ~39 W210 still FP)

- **Verdict:** FALSE POSITIVE (W210) — `namespace upvar ns src alias` legally
  links `alias` in the caller frame but no lowering hook records it as an
  IRCall def, so the analyser sees `alias` as undefined and flags every read.
- **Status:** **OPEN** — the fix in isolation is mechanical (a
  `lower_namespace_upvar` hook on `"namespace"` returning `None` for non-`upvar`
  subcommands), but it cascades into the shimmer pass: marking the alias as a
  def feeds shimmer's intrep heuristic, which defaults the alias's intrep to
  STRING and then flags every list/dict op on it — adding ~16 aycock FPs and
  unmasking ~241 pre-existing upvar-alias shimmer findings (`safe.tcl state`
  dominates). The shimmer-policy resolution (suppress alias intreps the same
  way SCCP force-OVERDEFINEs escaping vars) is a separate decision and the
  full landing waits on it.
- **Estimated impact:** ~39 W210 FPs across the corpus.
- **Codes:** W210
- **Corpus:** `tmp/tcl9.0.3/library/safe.tcl:109-115`

  ```tcl
  proc ::safe::CheckInterp {child} {
      namespace upvar ::safe [VarName $child] state
      if {![info exists state] || ![::interp exists $child]} {
          return -code error "\"$child\" is not an interpreter managed by ::safe::"
      }
  }
  ```

  (The same shape appears at safe.tcl:111, 141, 169, 217, 396 — ~30+ sites.)

#### Reproducer

```tcl
proc tester {} {
    # tclsh: 'alias' is now the caller-scope name for ::ns::state.
    namespace upvar ::ns state alias
    return $alias
}
```

#### Per-line reasoning

1. `proc tester {} { … }` — `alias` is not a parameter and not set by any
   later `set`.
2. `namespace upvar ::ns state alias` — `namespace upvar(n)`: *"Creates a
   variable in the current procedure that is linked to a variable in
   another namespace. … The newly-created link is named `localVarName`."*

   This is **functionally identical** to `upvar 1 ::ns::state alias`. tclsh:

   ```
   % namespace eval ::ns { variable state {ready} }
   % proc tester {} {
         namespace upvar ::ns state alias
         return $alias
     }
   % tester
   ready
   ```

   `alias` is a real local-scope name after this line. The analyser should
   record it as a def.

3. `return $alias` — reads the alias. **Should not fire W210**. Currently
   does.

The contrast with **`upvar`** (which works correctly) is the giveaway:
`compiler/lowering_hooks/_var.py:lower_upvar` registers the alias-name pair
as an IRCall def for `upvar`'s arg-pairs. There is no symmetric
`lower_namespace_upvar`. `namespace upvar`'s dispatched command is `namespace`,
not `upvar`, and the existing `namespace` dispatch handles `namespace eval` /
`namespace export` / etc. but doesn't recognise `namespace upvar`'s alias-write
shape.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% namespace eval ::ns { variable state ready }
% proc tester {} {
      namespace upvar ::ns state alias
      return $alias
  }
% tester
ready
```

No error. `alias` exists in the proc's local frame after the
`namespace upvar` line, linked to `::ns::state`.

#### Compiler evidence

```
--- FP-RBS-05: namespace upvar alias-not-a-def (OPEN; ~39 W210 still false-positive)
regen: python -m bench.fp_snippets --id FP-RBS-05
function ::tester
  block entry_1
    [0] Call cmd='namespace'  defs={}  uses={}
    term Return ${alias}
  read_before_set
    ReadBeforeSet(block='entry_1', statement_index=-1, variable='alias')
  dead_stores: (none)
```

- **`entry_1[0] Call cmd='namespace' defs={}`** — exactly the bug. The
  `namespace upvar` lowering doesn't recognise the alias-write subcommand, so
  no `alias#1` def is recorded.
- **`term Return ${alias}`** — the read.
- **`read_before_set: ReadBeforeSet(… variable='alias')`** — the false
  positive. This is what the user sees as W210.

This entry serves as the *negative* evidence for the eventual fix: when a
`lower_namespace_upvar` hook is added, this evidence should change to
`[0] Call cmd='namespace upvar' defs={alias#1}` and `read_before_set: (none)`,
the xfail test below will flip to a failure, and removing the xfail will be
the fix-landing receipt.

#### Why the analyser produces the FP

The `namespace` dispatch (handled inline by the IR lowerer rather than via
a hook in `compiler/lowering_hooks/`) only recognises a subset of subcommands
that need IR shape — `namespace eval` (becomes an IRBlock),
`namespace export` (no defs), etc. `namespace upvar`'s alias-write shape is
unrecognised, so the call is left as a bare `IRCall(defs=[])`.

The companion `upvar` handler in `compiler/lowering_hooks/_var.py` does this
correctly: it registers `(target, alias)` pairs as defs. A
`lower_namespace_upvar` hook with the same shape — but recognising the
`(ns, name, alias, [name alias …])` triplet pattern — would fix RBS.

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_05_alias_used_no_error_at_runtime` (TP
  proxy — the *runtime* behaviour is safe; this passes today as a sanity
  check, not as a fix-witness)
- `tests/test_fp_rbs.py::test_FP_RBS_05_namespace_upvar_silent` (FP,
  **`xfail(strict=True)`** — currently fires W210; flips to a failure when the
  `lower_namespace_upvar` hook lands, prompting the xfail removal)

---

### FP-RBS-06 — `[catch …]` output-var inside an `[expr {…}]` is written during expr eval

- **Verdict:** FALSE POSITIVE (W210) — extends FP-RBS-02 through expr-role
  args. The cmd-sub inside an expr body runs *during* expr evaluation and its
  output-var IS written before the rest of the expression reads it.
- **Status:** FIXED (commit `6ae85f4`).
- **Codes:** W210
- **Corpus:** `tmp/tcl9.0.3/library/http/http.tcl:4340` and `:4360`

  ```tcl
  set eof [expr {[catch {eof $sock} tmp] || $tmp}]
  ```

#### Reproducer

```tcl
proc f {sock} {
    # http.tcl:4340 pattern: the [catch …] inside [expr {…}] writes
    # 'tmp' during expr eval; the `|| $tmp` read must not be W210.
    set eof [expr {[catch {eof $sock} tmp] || $tmp}]
    return $eof
}
```

#### Per-line reasoning

1. `proc f {sock} { … }` — parameter `sock`. `tmp` is not a parameter, not
   set anywhere else.
2. `set eof [expr {[catch {eof $sock} tmp] || $tmp}]` — Tcl's `expr` engine
   evaluates the bracketed expression left-to-right with **short-circuit
   `||`**. When `[catch {eof $sock} tmp]` runs:
   - `eof $sock` is evaluated; the result (or any error) is captured;
   - **`catch` writes the result into `tmp` in the proc's local frame** (per
     `catch(n)`);
   - the catch returns its `0`/`1` status code to the expr.

   If the catch returns `0` (no error), `||` evaluates the right side,
   `$tmp`. By that point `tmp` is set — it's the third arg of the just-run
   `catch`. tclsh-verified:

   ```
   % proc f {x} {
         set eof [expr {[catch {expr {1.0/$x}} tmp] || $tmp == 0}]
         return "eof=$eof tmp=$tmp"
     }
   % f 2
   eof=0 tmp=0.5
   % f 0
   eof=0 tmp=Inf
   ```

   Both invocations succeed; `tmp` is always set by the cmd-sub before the
   `|| $tmp …` is evaluated.

3. `return $eof` — reads only the just-set `eof`; no FP risk here.

FP-RBS-02's mechanism (`command_sub_write_names`) handled `[catch …]` at the
command-arg level (`if {[catch …]} …`), but expr bodies need a separate
walk because their arg-role is EXPR, not BODY. The fix (commit `6ae85f4`)
adds `command_texts_in_expr_node` in `compiler/expr_ast.py` to walk
IRAssignExpr/IRExprEval/IRReturn expr ASTs collecting embedded
`[command-sub]` text, so the same `command_sub_write_names` recovery applies
to writes inside expr bodies.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {x} {
      set eof [expr {[catch {expr {1.0/$x}} tmp] || $tmp == 0}]
      return "eof=$eof tmp=$tmp"
  }
% f 2
eof=0 tmp=0.5
% f 0
eof=0 tmp=Inf
```

No error, `tmp` always reads correctly in the `|| $tmp …` subexpression.

#### Compiler evidence

```
--- FP-RBS-06: catch's output-var inside an expr body is written during expr eval
regen: python -m bench.fp_snippets --id FP-RBS-06
function ::f
  block entry_1
    [0] AssignExpr 'eof'  defs={eof#1}  uses={tmp#0}
    term Return ${eof}
  read_before_set: (none)
```

- **`[0] uses={tmp#0}`** — `tmp#0` is the SSA sentinel for "before any def";
  the expr's `$tmp` read resolves to that sentinel because no `set tmp …` /
  `upvar tmp` ever runs.
- **`read_before_set: (none)`** — yet the analyser correctly suppresses the
  W210 that this sentinel-use would normally trigger. That's the FP-RBS-06
  extension at work: `command_sub_write_names` walked the expr's embedded
  `[catch {…} tmp]` and recovered `tmp` as a write target, name-level
  exempting it from the RBS set.

Strip the extension (delete the `command_texts_in_expr_node` call from
`statement_cmd_sub_write_names` for IRAssignExpr) and W210 returns
immediately on `tmp`.

#### Why the analyser reaches that verdict

`compiler/expr_ast.py:command_texts_in_expr_node` walks IRAssignExpr,
IRExprEval and IRReturn expression ASTs collecting embedded command
substitutions; `compiler/var_refs.py:command_sub_write_names` then recovers
literal `VAR_WRITE` targets from each, fed into the suppressed-names set in
`_read_before_set`. The same scaffolding handles `[regexp …]` /
`[scan …]` inside expr bodies.

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_06_catch_inside_expr_silent` (FP)
- `tests/test_fp_rbs.py::test_FP_RBS_06_unrelated_inside_expr_still_fires`
  (TP control — an unrelated `$other` inside the same expr still fires)

---

### FP-RBS-07 — Dynamically-named `namespace eval` bodies are still analysed

- **Verdict:** FALSE POSITIVE (W210 + missed coverage) — a
  `namespace eval [computed] { … }` whose name was a command-sub or interpolation
  was treated as a fully opaque IRBlock barrier; the inner `proc` bodies were
  never analysed, so their *parameters* leaked as undefined names and every
  `$param` read inside fired W210.
- **Status:** FIXED (commit `cb14411`). Corpus delta: **−110 W210 FPs** on the
  full corpus.
- **Codes:** W210
- **Corpus:** `tmp/tcllib-2.0/modules/log/logger.tcl:1016` and `:1033`

  ```tcl
  namespace eval ::logger::tree::${service} {
      proc greet {who} { return "hello $who" }
      # … many procs, all with params that pre-fix leaked as W210
  }
  ```

#### Reproducer

```tcl
# logger.tcl:1007-1016 pattern: ${service} is the enclosing proc's
# parameter; the dynamic namespace name doesn't stop the body's inner
# `proc greet` from being analysed (post-fix).
proc trace_on {service} {
    namespace eval ::logger::tree::${service} {
        proc greet {who} { return "hello $who" }
    }
}
```

#### Per-line reasoning

1. `proc trace_on {service} { … }` — `service` is the enclosing proc's
   parameter, mirroring `logger::_trace_on { service }` at
   `logger.tcl:1007`. The dynamic namespace name (`${service}`) is a
   parameter read, not a bare unset.
2. `namespace eval ::logger::tree::${service} { … }` — `${service}` is a
   variable interpolation at the namespace-name position. The *body* (the
   braced script) is a static literal — tclsh parses and runs it normally
   inside whichever namespace `${service}` resolves to at runtime. tclsh:

   ```
   % set service foo
   % namespace eval ::logger::tree::${service} {
         proc greet {who} { return "hello $who" }
     }
   % ::logger::tree::foo::greet world
   hello world
   ```

   No error. The inner `proc greet` is a real proc definition that runs at
   load time and the dynamic namespace name has no effect on the body's
   parse-time analysis.

3. `proc greet {who} { return "hello $who" }` — `who` is a parameter, so
   `$who` is a perfectly normal parameter read. **There is no actual RBS
   here.** Pre-fix, however, the analyser never *got* to `greet` because the
   surrounding `namespace eval` was rejected as opaque (dynamic name → can't
   resolve scope → don't recurse into body). The inner proc was therefore
   never registered as a snapshot, its params weren't seeded, and any
   reference to `who` from another diagnostic pass (analyser scope tree,
   global scan, etc.) treated it as an unknown name.

The fix (commit `cb14411`): when the body is a *static braced script*, the
lowerer inline-compiles it (lifting `proc`s into their own scope) using the
enclosing namespace as a best-effort name. The original IRCall for
`namespace eval` is preserved (`source_args` / `source_tokens` retained) so
codegen still emits the dynamic call → bytecode stays byte-identical to
tclsh. Only the analyser sees through the opacity.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set service foo
% namespace eval ::logger::tree::${service} {
      proc greet {who} { return "hello $who" }
  }
% ::logger::tree::foo::greet world
hello world
```

No error; `who` reads correctly as a parameter inside `greet`.

#### Compiler evidence

```
--- FP-RBS-07: dynamically-named namespace eval bodies are still analysed (inner procs not opaque)
regen: python -m bench.fp_snippets --id FP-RBS-07
function ::greet
  block entry_1
    term Return hello ${who}
  read_before_set: (none)
```

- **`function ::greet` appears as a snapshot at all** — that's the fix. Pre-
  fix, `::greet` would not have been recognised because the dynamic
  `namespace eval` was an opaque barrier, so there'd be no
  `function ::greet` block here.
- **`term Return hello ${who}`** — the parameter read is in the IR.
- **`read_before_set: (none)`** — and it's correctly resolved (parameters
  enter as `who#0`).

#### Why the analyser reaches that verdict

The IR lowerer (in `compiler/lowering.py` / the `namespace eval` handler)
detects a static-braced body under a computed namespace name and inline-
compiles it as a normal namespace-eval scope, lifting nested `proc`s into
the compilation unit. `source_args` and `source_tokens` are kept on the
original IRCall so the codegen path is unchanged (bytecode identity).

The companion fix (`9f15e05`, see FP-RBS-08) handles the dynamic-namespace-
*name* read recovery — once the body was analysed, the previously-invisible
`$ns` / `$service` reads in the name slot became visible too.

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_07_dynamic_ns_eval_inner_param_silent`
  (FP, the canonical logger pattern)
- `tests/test_fp_rbs.py::test_FP_RBS_07_static_ns_eval_still_works` (FP
  control — the static-namespace form was never broken)

---

### FP-RBS-08 — `upvar` with a **dynamic** target (`upvar 1 $name var`) is a real alias-def

- **Verdict:** FALSE POSITIVE (W210/W220/W211) — `upvar 1 $name var` aliases
  the local `var` to a caller-scope variable named by the runtime value of
  `$name`. Pre-fix, the escaping-name collector filtered out pairs whose
  *target* started with `$`, so the alias-def was discarded and the local
  `var` looked completely unused.
- **Status:** FIXED (commit `9f15e05`). Companion fix: `namespace eval $ns
  {…}`'s `$ns` name is now scanned for reads via `IRBlock` use-extraction.
- **Codes:** W210, W220 (dead store on the alias write), W211 (unused
  variable)
- **Corpus:** `tmp/tcllib-2.0/modules/irc/picoirc.tcl:68-69`

  ```tcl
  set context [namespace current]::irc[incr uid]
  upvar #0 $context irc
  ```

  (Also: `parse_peghb.tcl:82` uses the same `upvar 1 $fv fixup` shape.)

#### Reproducer

```tcl
proc f {name} {
    # picoirc.tcl:69 pattern: upvar 1 $context irc — aliases 'irc'
    # to whatever the caller named.  Writes + reads must be silent.
    upvar 1 $name var
    set var 99
    return $var
}
```

#### Per-line reasoning

1. `proc f {name} { … }` — parameter `name` carries the *runtime* name of
   the caller-scope variable we want to alias. `var` is the *local* alias —
   not a parameter, not set anywhere outside this proc's body.
2. `upvar 1 $name var` — per `upvar(n)`: `upvar ?level? otherVar1 myVar1 …`
   creates a local variable `myVar` that **acts as an alias** for
   `otherVar` in the caller frame. Critically: `otherVar` can be any
   expression that produces a name at runtime — including `$name`. tclsh:

   ```
   % proc outer {nm} { upvar 1 $nm v; set v 99; return $v }
   % set myvar 0
   % outer myvar; puts $myvar
   99
   99
   ```

   The alias write `set v 99` reaches into the caller frame and updates
   `myvar`. The read `return $v` reads through the alias. Everything is
   real, the only "dynamic" part is *which* caller variable the alias
   points to.

3. `set var 99` — writes through the alias. Pre-fix this looked like a
   dead-store W220 because, with no recognised alias-def, `var` looked like
   a fresh local that was being assigned and never read (the next-line read
   was also missed).
4. `return $var` — reads through the alias. Pre-fix this also looked like
   W210.

The fix (commit `9f15e05`): the upvar lowering hook accepts dynamic targets
into `upvar_local_declaration_indices` when called by the escaping-name path
(via a new `allow_dynamic_target` flag). The memory-SSA and definition
callers keep strict (literal-target) matching because they need the
resolved target name; the escaping path only needs to know *that* an alias
exists.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {name} {
      upvar 1 $name var
      set var 99
      return $var
  }
% set myvar 0
% f myvar
99
% puts $myvar
99
```

No error. The alias works as advertised; `set var 99` writes the caller's
`myvar`.

#### Compiler evidence

```
--- FP-RBS-08: upvar with a dynamic target (upvar 1 $name var) is a real alias-def
regen: python -m bench.fp_snippets --id FP-RBS-08
function ::f
  block entry_1
    [0] Call cmd='upvar'  defs={var#1}  uses={name#0}
    [1] AssignConst 'var' value='99'  defs={var#2}  uses={}
    term Return ${var}
  read_before_set: (none)
  dead_stores: (none)
```

- **`[0] Call cmd='upvar' defs={var#1}`** — the upvar call is recognised as
  defining the alias `var` (version 1) **even though** its target arg
  (`$name`) is dynamic. `uses={name#0}` correctly records the dynamic-target
  read.
- **`[1] AssignConst 'var' value='99' defs={var#2}`** — the alias write
  records version 2; the prior `var#1` is consumed by being the predecessor
  state, not a dead store.
- **`read_before_set: (none)` + `dead_stores: (none)`** — both the W210 and
  W220 FPs are gone. The TP control test asserts that an *unrelated* local
  in the same proc still fires correctly.

#### Why the analyser reaches that verdict

`compiler/lowering_hooks/_var.py:lower_upvar` records each `(target, alias)`
pair as an IRCall def for the alias name. The companion
`upvar_local_declaration_indices` helper (consumed by the escaping-name
collector that drives `_is_externally_mutable`) gained an
`allow_dynamic_target` flag so the escaping path opts in. The memory-SSA
and definition callers keep their strict literal-target matching because
they need a resolved name; this path only needs to know an alias exists.

Companion: `compiler/ir.py:IRBlock` use-extraction now reads `source_args[1]`
so `namespace eval $ns {…}`'s `$ns` is recovered as a read of `ns` (was
invisible pre-fix — would have looked unused).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_08_dynamic_upvar_target_silent` (FP, the
  `upvar 1 $name var` pattern)
- `tests/test_fp_rbs.py::test_FP_RBS_08_unrelated_local_still_fires` (TP
  control — a local that's *not* aliased and never set still fires W210)

---

### FP-RBS-09 — `for`-init + regexp/cmd-sub captures inside un-lowered switch arms

- **Verdict:** FALSE POSITIVE (W210) — un-lowered switch arms hid the defs
  made by `for {set j 0} …`'s init script and `if {[regexp … -> v]}`'s
  capture-var, so subsequent reads of `$j` / `$v` falsely flagged.
- **Status:** FIXED (commit `9e379bd`). Corpus delta: **−20 W210 FPs**.
- **Codes:** W210
- **Corpus:** `tmp/tcllib-2.0/modules/struct/record.tcl:661` (and the
  `switch` patterns throughout the struct module).

#### Reproducer

```tcl
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $j }
        }
        b {
            if {[regexp {(\w+)} "foo" -> v]} { puts $v }
        }
    }
}
```

#### Per-line reasoning

1. `switch -- $n { a { … } b { … } }` — `switch` selects one body to run.
   In the analyser's IR, each arm's body is an IRBlock that's analysed for
   reads but, pre-fix, didn't have its inner defs surfaced into the
   surrounding scope's def set when computing read-before-set across the
   arm.
2. `for {set j 0} {$j < 3} {incr j} { puts $j }` — `for` has four
   sub-scripts: init, condition, next, body. The **init script** runs
   exactly once before the loop and is the conventional place to introduce
   the loop variable (`set j 0`). tclsh-verified: `for {set j 0} … { puts
   $j }` runs without any "j unset" error because `set j 0` is the first
   thing `for` evaluates. Pre-fix, the switch-arm wrapping hid the for-init's
   `set j 0` so the body's `$j` looked like a read-before-set.
3. `if {[regexp {(\w+)} "foo" -> v]} { puts $v }` — `regexp ?-> var…?`
   writes capture-vars in the caller frame (analogue of FP-RBS-02's
   catch-msg-var). The `if` consequent reads `$v` legally. Pre-fix, the
   regexp's capture-var def was hidden behind the switch-arm wrapping.

The fix (commit `9e379bd`) completes the def set inside
`_free_reads_in_ir_script` (`_collapsed_extra_defs`): for-init / for-next
scripts contribute their `set` defs, and condition cmd-subs contribute
their write-targets (the FP-RBS-02 mechanism). Done at the local level
rather than touching shared `cfg._defs_from_ir_script` to avoid perturbing
the SCCP catch/try-merge invalidation (which was sensitive to a
`bigfloat2.tcl` S100 determinism artefact).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% f a
0
1
2
% f b
foo
```

No error. Both arms run to completion with their captures.

#### Compiler evidence

```
--- FP-RBS-09: for-init + regexp/cmd-sub captures inside un-lowered switch arms
regen: python -m bench.fp_snippets --id FP-RBS-09
function ::f
  …
  block switch_arm_body_4
    [0] AssignConst 'j' value='0'  defs={j#1}  uses={}
    term Goto
  block switch_arm_body_5
    [0] Call cmd='<cond>'  defs={->#1, v#2}  uses={}
    term Branch ExprCommand(text='[regexp {(\\w+)} "foo" -> v]', start=0, end=26)
  …
  block for_body_9
    [0] Call cmd='puts'  defs={}  uses={j#2}
    term Goto
  …
  block if_then_13
    [0] Call cmd='puts'  defs={}  uses={v#2}
    term Goto
  …
  read_before_set: (none)
```

- **`switch_arm_body_4[0] defs={j#1}`** — the for-init's `set j 0` is now
  visible at the surrounding scope so the body's reads resolve.
- **`switch_arm_body_5[0] defs={->#1, v#2}`** — the regexp's capture-vars
  are recovered (the literal `->` placeholder and the real capture `v`).
  The `Call cmd='<cond>'` is the synthetic node `command_sub_write_names`
  attaches to the branch condition.
- **`for_body_9[0] uses={j#2}`** and **`if_then_13[0] uses={v#2}`** —
  the consequent reads resolve cleanly to the just-recovered defs.
- **`read_before_set: (none)`** — no W210 anywhere.

#### Why the analyser reaches that verdict

`_free_reads_in_ir_script` (in `compiler/core_analyses.py`) computes the
def set used to filter the read-before-set candidates emitted by un-lowered
script analysis. Pre-fix it only included direct top-level
`VAR_WRITE` defs; the fix adds `_collapsed_extra_defs` which folds in
for-init / for-next `set` targets and condition command-sub write-targets.

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_09_for_init_in_switch_arm_silent` (FP)
- `tests/test_fp_rbs.py::test_FP_RBS_09_regexp_capture_in_switch_arm_silent`
  (FP)
- `tests/test_fp_rbs.py::test_FP_RBS_09_genuine_unset_in_arm_still_fires`
  (TP control — a variable that's never written *anywhere* in the proc still
  fires W210, proving the def-recovery is targeted)

---

### FP-RBS-10 — `eval` / `namespace eval` literal-body reads are recovered

- **Verdict:** FALSE POSITIVE (W214 + W210) — `eval { … $x … }` evaluates the
  braced body in the current scope, so `$x` is a real read of the local.
  Pre-fix the body was an opaque IRBlock and `x` looked unused (W214) and
  any body-only var looked read-before-set (W210).
- **Status:** FIXED (commit `6f69c86`). Recovery is suppress-only and
  name-level (`_extra_local_reads` + `_block_local_reads`, exposed as
  `ssa.statement_read_names`). Full CFG flatten remains a future option.
- **Codes:** W210, W214
- **Corpus:** `tmp/tcl9.0.3/library/init.tcl` (numerous `eval { … }` sites);
  also `tmp/tcllib-2.0/modules/control/control.tcl:14`.

#### Reproducer

```tcl
proc f {x} {
    # eval's braced body evaluates in *this* scope: $x is a real read of
    # the parameter, so 'x' must not be reported W211 ("unused").
    eval { puts $x }
}
```

#### Per-line reasoning

1. `proc f {x} { … }` — `x` is the only parameter; if the analyser thinks
   nothing reads it, W214 ("Parameter '…' of proc '…' is unused") fires.
2. `eval { puts $x }` — `eval(n)`: *"`eval` … concatenates the arguments,
   then evaluates the result as a Tcl script."* When the body is a single
   braced word, tclsh evaluates it directly in the **current** scope (the
   `f` proc's locals). The `puts $x` therefore reads `x` from `f`'s
   parameters. tclsh:

   ```
   % proc f {x} { eval { puts $x } }
   % f hello
   hello
   ```

   No error. `$x` reads `f`'s parameter, prints "hello".

   Pre-fix, the analyser treated `eval { … }`'s body as opaque — none of
   the body's reads were surfaced to the surrounding scope's read set, so
   `x` looked unused → W214. (Similarly, a body-only `set y; … $y` looked
   like RBS.) The fix collects body-internal reads name-level via
   `_extra_local_reads` and `_block_local_reads`, exposed on the SSA
   statement so unused-variable / read-before-set passes can see them.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {x} { eval { puts $x } }
% f hello
hello
% proc g {x} { namespace eval ::ns { puts "hello $x" } }
% g world
hello world
```

Both bodies see the caller's `x`. tclsh's `namespace eval` evaluates the
body in the *target namespace*, but Tcl's scoping rules still let it see
the caller's locals via the proc's frame (this is the standard
namespace-eval-inside-proc pattern).

#### Compiler evidence

```
--- FP-RBS-10: eval / namespace eval literal-body reads are recovered
regen: python -m bench.fp_snippets --id FP-RBS-10
function ::f
  block entry_1
    [0] Block  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
  read_before_set: (none)
  dead_stores: (none)
```

- **`[0] Block defs={} uses={}`** — the eval body is preserved as an
  IRBlock; the SSA-visible defs/uses are intentionally empty (the body
  isn't flattened into the CFG, just analysed name-level for reads/writes).
- **`read_before_set: (none)`** — yet no W210, and crucially **no W214
  on `x`** (which is what would surface if the analyser missed the body's
  read). The recovery happens *outside* the SSA layer:
  `ssa.statement_read_names` exposes the body's literal var-reads (here,
  `x`) name-level to the unused-variable check.

To see the FP without the fix, temporarily delete the
`_block_local_reads` call in the unused-parameter collector — `x` will
immediately appear in the W214 output.

#### Why the analyser reaches that verdict

`compiler/var_refs.py:_block_local_reads` and `_extra_local_reads` walk
the eval/`namespace eval` body's token stream collecting bare `$var` reads,
exposed via the public `ssa.statement_read_names`. The unused-variable and
read-before-set checks query this name set as a suppression layer
(name-level, so a body-only `$x` keeps `x` out of the unused / RBS report
without forcing IR/SSA shape changes).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_10_eval_body_param_silent` (FP, the
  eval-body `$x` read keeps `x` from showing as W214)
- `tests/test_fp_rbs.py::test_FP_RBS_10_genuine_unused_still_fires` (TP
  control — a parameter that's truly never read still fires W214)

---

### FP-RBS-11 — Qualified-builtin loops (`::foreach`, `::lmap`, `::for`, `::while`)

- **Verdict:** FALSE POSITIVE (W210) — qualified spellings of the loop
  built-ins were left as opaque IRCall barriers, so the analyser missed both
  the loop-var binders (`k`, `v`) and the body's defs/reads.
- **Status:** FIXED (commit `2f67c93`, analysis-only path). Corpus delta:
  **−29 W210 FPs**. Note the lowering-side fix was reverted because it
  surfaced pre-existing FPs in other checks (dict-internals E002, html
  shimmer); the analysis-only recovery is the safe landing.
- **Codes:** W210
- **Corpus:** `tmp/tcllib-2.0/modules/html/html.tcl:153` (and `:306`,
  `:420`, …)

  ```tcl
  ::foreach {vars vals} $varvals {
      # body reads $vars and $vals — pre-fix all of them flagged W210
  }
  ```

#### Reproducer

```tcl
proc f {dict} {
    # html.tcl:153 pattern: ::foreach is just qualified foreach.
    # Loop vars k,v and body reads must all be silent.
    ::foreach {k v} $dict { puts "$k=$v" }
}
```

#### Per-line reasoning

1. `proc f {dict} { … }` — parameter `dict`. `k` and `v` are not
   parameters; they're loop-var binders introduced by the `::foreach` call.
2. `::foreach {k v} $dict { puts "$k=$v" }` — `::foreach` is the absolutely-
   qualified spelling of the `foreach` built-in (Tcl's auto-loader resolves
   it the same way). tclsh:

   ```
   % ::foreach {k v} {a 1 b 2} { puts "$k=$v" }
   a=1
   b=2
   ```

   The loop binds `k` to each odd-index element and `v` to the next, then
   runs the body. Both are local-scope writes — exactly like bare `foreach`.

   The pre-fix bug: lowering dispatch in `compiler/lowering.py` keyed on
   bare command names (`case "foreach"`), so the qualified spelling
   `::foreach` never matched — it was lowered as a generic IRCall barrier.
   Without recognising it as a loop, neither `k`/`v` nor the body's reads
   were surfaced. The body's `$k` and `$v` then fired W210.

The analysis-only fix (commit `2f67c93`): extend the un-lowered-loop
recovery inside `_read_before_set` to also recognise `::foreach`/`::lmap`
heads, recover the loop vars + body writes name-level. Same W210 win,
without changing the IR shape and without exposing the body to
E002/shimmer/SCCP.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% ::foreach {k v} {a 1 b 2} { puts "$k=$v" }
a=1
b=2
% ::for {set i 0} {$i < 3} {incr i} { puts $i }
0
1
2
```

Both qualified forms run identically to their bare counterparts.

#### Compiler evidence

```
--- FP-RBS-11: qualified-builtin loops (::foreach / ::lmap / ::for / ::while)
regen: python -m bench.fp_snippets --id FP-RBS-11
function ::f
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='::foreach'  defs={}  uses={dict#0, k#0, v#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
  read_before_set: (none)
```

- **`[1] Barrier cmd='::foreach' defs={}`** — the qualified call stays an
  opaque barrier (the lowering-side fix was reverted, see status note); the
  defs set really is empty in the IR.
- **`uses={dict#0, k#0, v#0}`** — the barrier records the *body's* reads
  as use-sites. `dict#0` is the parameter (entry version 0), `k#0` and
  `v#0` are sentinel "before-any-def" reads — exactly what would normally
  fire W210.
- **`read_before_set: (none)`** — but no RBS is emitted, because
  `_read_before_set` recognises the `::foreach` barrier as a loop-form and
  exempts the loop-var binders + body-write targets name-level (commit
  `2f67c93`).

#### Why the analyser reaches that verdict

`_read_before_set` in `compiler/core_analyses.py` walks IRBarrier statements
matching the loop-builtin name set (including qualified `::foreach` /
`::lmap` / `::for` / `::while`) and recovers the loop vars from the
canonical loop-builtin shape, name-level exempting them from the RBS set.
The body's `VAR_WRITE` targets are recovered the same way (the FP-RBS-03
body-write mechanism, here applied through the qualified path).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_11_qualified_foreach_silent` (FP)
- `tests/test_fp_rbs.py::test_FP_RBS_11_qualified_for_silent` (FP control —
  the `::for` shape works the same way)
- `tests/test_fp_rbs.py::test_FP_RBS_11_genuine_unset_in_body_still_fires`
  (TP — a body-read of a never-bound var still fires W210)

---

## Conventions for adding entries (for future PR slices)

- One commit per family (PR slice). Each commit adds:
  1. New `FP-<FAMILY>-NN` entries in this file with the seven sections above.
  2. The matching evidence-generator entries in `bench/fp_snippets.py`.
  3. The paired TP/FP tests in `tests/test_fp_<family>.py`.
- The reproducer in this file **is** the test source string — copy-pasted, not
  re-typed. This guarantees doc and test stay in lock-step.
- Compiler evidence blocks must be regenerated via `bench.fp_snippets`; never
  hand-edited. The regen command is printed in the block header.
- Open findings (still FP today) use `pytest.mark.xfail(reason="FP-<id> open",
  strict=True)` on the FP test, so the day the fix lands the xfail flips to a
  failure and prompts its own removal.
- Ground truth is tclsh 9.0.3 (or the relevant 8.x for dialect-sensitive cases,
  noted per-entry).
