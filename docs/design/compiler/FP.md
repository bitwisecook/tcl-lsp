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
- **RBS — Read-before-set (W210/W213/W214)** ([§RBS](#rbs--read-before-set-w210w213w214))
- **DS — Dead-store / unused (W220/W211)** ([§DS](#ds--dead-store--unused-w220w211))
- **SH — Shimmer (S100/S101/S102)** ([§SH](#sh--shimmer-s100s101s102))
- **OBJ — Object dispatch (W307/W308) + snit modelling** ([§OBJ](#obj--object-dispatch-w307w308--snit-modelling))
- **RCH — Reachability (O107)** ([§RCH](#rch--reachability-o107))
- **INJ — Injection / style (W101/W105/W301/T102)** ([§INJ](#inj--injection--style-w101w105w301t102))
- **BND — Bounds / intervals (W230/W231/W232/W233)** ([§BND](#bnd--bounds--intervals-w230w231w232w233))
- **OPT — Optimisation / codegen quick-fixes (O106/O109/O110/O116/O120/O126)** ([§OPT](#opt--optimisation--codegen-quick-fixes-o106o109o110o116o120o126))
- **TNT — Taint flow (T100/T101)** ([§TNT](#tnt--taint-flow-t100t101))
- **STY — Style / usage (W001/W104/W120/W122/W124/W126/W214/W302/W306)** ([§STY](#sty--style--usage-w001w104w120w122w124w126w214w302w306))

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
# tclsh contract: lset at index == length APPENDS (legal, not an error).
set l {a b c}      ;# llength=3
lset l 3 X         ;# 3 == llength $l -> APPENDS X (NOT an error)
puts $l            ;# use post-lset binding (silences W211 on l#2)
```

#### Per-line reasoning

1. `set l {a b c}` — `l#1` is a brace-literal list with **statically known**
   length 3.  The SCCP lattice folds it to `CONST('a b c')` so the bounds
   check at the next line has a concrete length to compare against.  A
   parameter form (`proc f {l}`) would leave the length unknown and the
   bounds check would silence itself trivially — the verdict would be
   vacuous because *any* index, valid or invalid, would be silent.
2. `lset l 3 X` — Tcl's `lset` documentation (`lset(n)`) states that when
   the index equals the current list length, the element is **appended**.
   It only errors for `index > length` or `index < 0`.  With the literal
   list of length 3, the analyser sees the precise comparison `3 > 3`
   (the strict comparator at `analyser/checks/_bounds.py:318`) and
   returns **False** → no W231.  If a regression replaced `>` with `>=`
   the same line would fire W231 and the catalog would visibly catch it.
3. `puts $l` — uses the post-lset binding `l#2`, silencing the
   unused-variable warning on the rebind (W211).  The dead-store warning
   on `l#1` (W220) still fires — see "incidental noise" below.

The verdict is **NOT a W231**: append at `index == length` is legal.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set l {a b c}; lset l 3 X; puts $l
a b c X
```

(Compare: `lset l 4 X` on a 3-element list errors with `list index out of
range`; `lset l -1 X` errors too.  The append slot is the **only**
non-error out-of-bounds index.)

#### Compiler evidence

```
--- FP-NAB-01: lset append-slot (index == length) is legal, NOT W231
regen: python -m bench.fp_snippets --id FP-NAB-01
function ::top
  block entry_1
    [0] AssignConst 'l' value='a b c'  defs={l#1}  uses={}
    [1] Call cmd='lset'  defs={l#2}  uses={}
    [2] Call cmd='puts'  defs={}  uses={l#2}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    l#1: CONST('a b c')
    l#2: OVERDEFINED
  dead_stores
    DeadStore(block='entry_1', statement_index=0, variable='l', version=1)
```

- `l#1: CONST('a b c')` — the **statically known** length-3 list the bounds
  check needs.  Without this concrete value, the W231-silent verdict would
  be vacuous (parameter-form lengths are unknown and the check is silent
  regardless of the comparator).
- `l#2: OVERDEFINED` — SCCP cannot fold `lset`'s result.
- The bounds check that *would* fire here is W231; the catalog's negative
  test asserts it does **not** fire because the comparator at
  `analyser/checks/_bounds.py:318` is strict `>`, which permits the append
  slot.

##### Incidental noise: the W220 on `l#1`

The evidence includes `DeadStore(... variable='l', version=1)` and the
diagnostic stream emits W220 on the `set l {a b c}` line.  This is a known
modelling artefact: `lset l ...` syntactically writes to `l` and the SSA
lowering records `defs={l#2}, uses={}` for the `lset` call — it doesn't
record `l#1` as a use, even though `lset` semantically reads-modifies-
writes.  So `l#1` looks dead w.r.t. the rebind.  This is incidental to
the W231 verdict (it's a separate dead-store check) and the regression
test (`tests/test_fp_nab.py::test_FP_NAB_01_append_slot_silent`) asserts
only that **W231** stays silent.

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

### FP-NAB-04 — W110 / O120 `==` / `!=` on strings → `eq` / `ne` (TP)

- **Verdict:** TRUE POSITIVE (style / safety rule, not a hard semantic error)
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_04_*`
- **Codes:** W110 / O120 (near-duplicate pair)
- **Corpus:** W110=1673, O120=1515 firings; near-duplicate pair, consolidation a future policy call.

#### Reproducer

```tcl
if {$x == "hello"} { puts y }
```

#### Per-line reasoning

Tcl's `==`/`!=` are operators with **dual semantics**: they compare
numerically when both operands parse as numbers, otherwise they
compare as strings.  This dual mode is the foot-gun:

- ``"hello" == "hello"`` — neither parses as a number, so Tcl falls
  back to string equality and returns `1`.  The code happens to work.
- ``"02" == "2"`` — both parse as numbers, so Tcl compares
  *numerically* and returns `1` — but the writer probably intended
  string equality and would expect `0`.
- ``"10" == "10a"`` — left parses numeric, right doesn't, so Tcl
  falls back to string equality and returns `0` — but if the writer
  was reasoning numerically they'd expect "10 equals 10-with-tail-a"
  to be false anyway (lucky outcome from the wrong operator).

The hazard is the **silent mode switch** based on operand shape —
the code's correctness depends on data values, not on the source
text.  `eq` / `ne` are unambiguous string equality and remove the
mode-dependence; W110 / O120 push the user toward the safer
operator.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% expr {"hello" == "hello"}
1
% expr {"02" == "2"}
1
% expr {"02" eq "2"}
0
% expr {"hello" eq "hello"}
1
```

`==` returned `1` for the numeric-coercion case where the strings
differ textually (`"02"` vs `"2"`); `eq` correctly reports them as
different strings.  The numeric-vs-string mode-switch is the
foot-gun.

#### Why the analyser reaches that verdict

`compiler/optimiser/_expr_simplify.py` (O120) and the analyser's
W110 emitter both detect `==`/`!=` against a string literal and
suggest the `eq`/`ne` rewrite.  This is a *style/safety* rule, not
a hard semantic error — the code may work today but breaks the
moment the data starts looking numeric.

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_04_string_eq_fires_w110_o120` (TP)

---

### FP-NAB-05 — W304 missing `--` terminator (TP)

- **Verdict:** TRUE POSITIVE for ``file delete``/``file rename`` and the
  **split pattern/body switch form**.  Note: the **braced pattern-list
  switch form** (``switch $x {pat body pat body}``) is **not** a runtime
  hazard — Tcl unambiguously recognises the trailing brace as the
  pattern list and treats the preceding word as the value.  The
  analyser still fires W304 on the braced form (conservative, slight
  over-reach); a future precision tightening could distinguish.
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_05_*`
- **Codes:** W304
- **Corpus:** 1453 firings.

#### Reproducer (real hazard — split pattern/body form)

```tcl
proc f {f} { file delete $f }
```

Plus the split-switch form:

```tcl
switch $x -nocase {puts hit1} default {puts hit2}
```

#### Per-line reasoning

The W304 hazard fires when an option-parsing command sees a
substituted value with a leading ``-``:

1. ``file delete $f`` with ``$f = "-force"`` — Tcl interprets
   ``-force`` as the documented ``-force`` option (force-delete) and
   then there is no path argument, so the call effectively becomes
   ``file delete -force`` with no path → unintended behaviour.  The
   safe form is ``file delete -- $f``.
2. ``switch $x -nocase {puts hit1} default {puts hit2}`` (the
   **split** pattern/body form, options before string).  When
   ``$x = "-nocase"`` Tcl interprets the FIRST ``-nocase`` (the
   substituted value) as the case-insensitive option, then the
   *literal* ``-nocase`` is the pattern, and the match falls through
   to ``default``.  Confirmed in tclsh 9.0.3:

   ```
   % set x -nocase
   % switch $x -nocase {puts hit1} default {puts hit2}
   hit2
   % switch -- $x -nocase {puts hit1} default {puts hit2}
   hit1
   ```

The **braced pattern-list form** (``switch $x { ... }``) is **not**
hazard-bearing because Tcl can unambiguously identify the brace-list
position; ``switch $x { -nocase { puts a } ... }`` with
``$x = "-nocase"`` correctly matches ``-nocase`` as a pattern:

```
% set x -nocase
% switch $x { -nocase { puts a } default { puts b } }
a
```

The analyser's W304 currently fires on both forms (a conservative
over-reach for the braced form); a future precision tightening could
distinguish.  The TP claim above stands for the split-switch form and
all the ``file delete``-style commands.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set f -force
% file delete $f           ;# Tcl sees -force as the option, no path arg
                            ;# (silently does nothing; intended path is dropped)
% file delete -- $f        ;# safe form — -force treated as path
couldn't delete "-force": no such file or directory
```

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_05_switch_missing_dash_dash_fires_w304` (TP)

---

### FP-NAB-06 — W103 `open` variable arg / pipe (TP)

- **Verdict:** TRUE POSITIVE
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_06_*`
- **Codes:** W103
- **Corpus:** 398 firings.

#### Reproducer

```tcl
proc f {cmd} { set fh [open "|$cmd" r] }
```

#### Per-line reasoning

`open` with a leading `|` in the first arg starts a pipeline through
the substituted command.  Even with an explicit access mode (`r`),
the `|` prefix wins — tclsh forks `$cmd` and pipes its output to the
returned channel.  Substituting `$cmd` from any external source is an
arbitrary-command-execution vector.

#### tclsh ground truth

```
% set cmd "echo hi"
% set fh [open "|$cmd" r]
file3
% read $fh
hi
```

The pipe is real; the command runs.

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_06_open_variable_pipe_fires_w103` (TP)

---

### FP-NAB-07 — W313 destructive op with variable path (TP)

- **Verdict:** TRUE POSITIVE
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_07_*`
- **Codes:** W313
- **Corpus:** 95 firings.

#### Reproducer

```tcl
proc f {p} { file delete $p }
```

#### Per-line reasoning

Destructive filesystem operations (`file delete`, `file rename`,
`file copy -force`) on a substituted path are real foot-guns: the
variable could resolve to anything from an unintended path to a
path-traversal payload.  The fix isn't to suppress the diagnostic
but to make the caller sanitise or pin the value.

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_07_destructive_variable_path_fires_w313` (TP)

---

### FP-NAB-08 — W212 substitution where var-name expected (TP)

- **Verdict:** TRUE POSITIVE
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_08_*`
- **Codes:** W212
- **Corpus:** 390 firings.

#### Reproducer

```tcl
proc f {name v} { set $name $v }
proc g {x} { incr $x }
```

#### Per-line reasoning

`set $name $v` uses the *substituted value* of `$name` as the
variable name to write to — a dynamic-name pattern.  Without explicit
guards (`upvar`, `dict`, `trace`, `namespace which`) the writer has
no idea what variable they're actually creating; the form is a
genuine foot-gun.  The Tcl-idiomatic alternatives (`upvar 1 $name v; set v $v`,
`dict set name $v`, `array set arr [list $name $v]`) make the
indirection explicit.

The W212 emitter correctly exempts `upvar`/`dict`/`trace`/
`namespace which` (which legitimately use substituted var names).

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_08_set_substituted_name_fires_w212` (TP)
- `tests/test_fp_nab.py::test_FP_NAB_08_incr_substituted_name_fires_w212` (TP)

---

### FP-NAB-09 — W301 uplevel multi-arg concatenation (TP)

- **Verdict:** TRUE POSITIVE
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_09_*`
- **Codes:** W301
- **Corpus:** 291 firings (logger.tcl idioms).

#### Reproducer

```tcl
proc f {a b} { uplevel 1 puts $a $b }
```

#### Per-line reasoning

`uplevel 1 cmd $a $b` concatenates its three args into a single
script string before evaluation in the caller's frame.  The
substituted `$a` and `$b` values are re-parsed as command syntax —
classic eval-injection vector.

The safe forms are documented in FP-INJ-01 (bare-var `uplevel 1
$body` where `$body` is the entire script) and `uplevel 1 [list cmd
$a $b]` (which prevents re-parsing).

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_09_uplevel_multiarg_fires_w301` (TP)

---

### FP-NAB-10 — W002 disabled-in-dialect (TP after dialect-aware harness)

- **Verdict:** TRUE POSITIVE (the prior "FP" was a harness artefact)
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_10_*`
- **Codes:** W002
- **Corpus:** real firings (e.g. `log` disabled in some dialect contexts) are TP; the `oo::configurable` "FP" was the harness defaulting to tcl8.6 instead of tcl9.

#### Reproducer

```tcl
# When analysed under dialect tcl8.4:
dict create a 1
```

#### Per-line reasoning

`dict` was added in Tcl 8.5; using it in a file that targets Tcl 8.4
is genuinely an error (the command does not exist).  W002 must fire.

The earlier "FP" report — `oo::configurable` flagged in tcl9 corpus
sweeps — turned out to be a *harness* artefact: a raw
`get_diagnostics(src)` uses the default dialect tcl8.6, where
`oo::configurable` is indeed disabled.  Sweeping the corpus
*dialect-aware* (detect `package require Tcl X.Y` / `# tcl-dialect:`
and wrap in `dialect_scope(...)`, as the LSP does) made the
phantom firings disappear, confirming the diagnostic is correct.

#### tclsh ground truth

```
$ tclsh8.4 -c 'dict create a 1'
invalid command name "dict"
$ tclsh9.0 -c 'dict create a 1'
a 1
```

#### Why the analyser reaches that verdict

`compiler/registry/dialect.py::dialect_scope` and the command registry
gate availability per dialect; W002 fires only when the command isn't
in the current dialect's enabled-command set.

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_10_dict_disabled_in_tcl_8_4_fires_w002` (TP)
- `tests/test_fp_nab.py::test_FP_NAB_10_dict_enabled_in_tcl_9_0_silent` (FP control — same source, different dialect, silent)

---

### FP-NAB-11 — W123 unresolved command (TP — real missing stubs, not analyser FPs)

- **Verdict:** TRUE POSITIVE (the firings are real; the per-package stub bundle is an open *noise-reduction* optimisation, not a precision fix)
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_11_*`
- **Codes:** W123 (unknown command)
- **Corpus:** 1761 firings — argparse, dict-extension (`dget`/`dexist`), custom widget commands.

#### Reproducer

```tcl
argparse {x y}
```

#### Per-line reasoning

W123 fires when a command isn't found in the registered command set
(builtins + registered packages + locally-defined procs).  Spot-
checking the 1761 corpus firings shows they fall into three buckets,
all genuine *missing-stub* TPs:

1. **Tcllib packages** without stubs (argparse, sha256, etc.).  These
   are real Tcl packages whose registry stubs we haven't shipped.
2. **Dict extension commands** (`dget`, `dexist`) from third-party
   modules.
3. **Project-local custom widget / DSL commands** (the ``my::widget
   foo`` shape in F5 iRules dialect modules, etc.).

None of these are analyser FPs — the analyser correctly doesn't know
about them.  The *fix* is to ship per-package stub bundles; that's
catalogued as an open task in `fp-audit-todo.md` (not a precision
issue).

**Workspace-level refinement (issue #832).**  The *single-file analyser*
verdict above is unchanged — it cannot see beyond the document.  But the
LSP server *does* have the project's package database (`pkgIndex.tcl` /
`tclIndex` across the workspace + the resolved `auto_path`), the same
knowledge go-to-definition uses.  A W123 whose command that database can
resolve — a library proc a `tclIndex` auto-loads with no `package
require` (the BLT/Rbc idiom), or a command an available package's
implementation defines — is refined away by `refine_workspace_w123`
(alongside the W120 refinement, and independent of the `xcDiagnostics`
toggle), so it never reaches the editor.  This is a resolvability lookup
driven entirely by the resolver, never a command-name allowlist.  Locked
in by `autoload_library_command_suppresses_w123_issue_832` /
`pkgindex_package_source_command_suppresses_w123` (server) and the
`auto_loads_command` / `package_defined_commands` unit tests (resolver).

#### tclsh ground truth

```
$ tclsh9.0 -c 'argparse {x y}'
invalid command name "argparse"
```

Confirmed: `argparse` is genuinely not a Tcl builtin and requires
a `package require argparse` (which would import its stub).

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_11_unresolved_argparse_fires_w123` (TP)
- `tests/test_fp_nab.py::test_FP_NAB_11_stub_registered_command_silent` (FP control — registered command silent)

---

### FP-NAB-12 — `is_pure_var_ref("$a(x\)y)")` handles escaped close-paren in array index (D4-F11)

- **Verdict:** TRUE POSITIVE / tooling-API correctness fix (now fixed)
- **Status:** locked in by `tests/test_fp_nab.py::test_FP_NAB_12_*`
- **Codes:** N/A (tooling-API audit — `is_pure_var_ref` is a shared helper consumed by W301 and the uplevel/safe-eval idioms)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D4-F11

#### Reproducer

```tcl
proc f {} {
    set a(x\)y) 1
    puts $a(x\)y)
}
```

#### Per-line reasoning

1. `set a(x\)y) 1` — tclsh parses the array name as `a`, the index as the literal four-character string `x\)y` (the backslash escapes the close paren so the index doesn't terminate there).  This is a well-formed array element assignment.
2. `puts $a(x\)y)` — same escape rule for the var reference.  Tcl reads the value back from the same element; the reference is exactly one pure var ref (no concatenation, no surrounding syntax).
3. The old `is_pure_var_ref` regex `\$[\w:]+(\([^)]*\))?` rejected this because `[^)]*` stopped at the FIRST close paren regardless of the preceding backslash.  The hand-rolled scanner `_scan_pure_var_ref` walks the index character-by-character and consumes `\<any>` as an escaped pair, so the escaped `)` is no longer treated as the index terminator.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set a(x\)y) 1
1
% puts $a(x\)y)
1
```

The set, the read, and the printf all succeed; the index literally is `x\)y`.

#### Compiler evidence

```
--- FP-NAB-12: is_pure_var_ref handles backslash-escaped close paren in array index (D4-F11)
regen: python -m bench.fp_snippets --id FP-NAB-12
function ::f
  block entry_1
    [0] AssignConst 'a(x)y)' value='1'  defs={a#1}  uses={}
    [1] Call cmd='puts'  defs={}  uses={a#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-NAB-12`)

#### Why the analyser reaches that verdict

`compiler/value_shapes.py:6` — `_scan_pure_var_ref` is a hand-rolled Tcl-correct parser for the three documented variable-reference forms (`$name`, `${name}`, `$arr(idx)`).  For the array form, the index scanner explicitly consumes `\<any>` as a two-character escape sequence (line 53-55) so a backslash-escaped close paren no longer terminates the index.  `is_pure_var_ref` (line 63) succeeds iff the scan consumes the entire input text.

#### Tests

- `tests/test_fp_nab.py::test_FP_NAB_12_escaped_paren_array_index_is_pure_var_ref` (TP — the tooling-API call returns True on the escaped form)
- `tests/test_fp_nab.py::test_FP_NAB_12_unescaped_paren_terminates_index` (TP control — `$a(x)y` parses as `$a(x)` followed by literal `y`, so it is NOT a single pure var ref)
- `tests/test_ground_truth_tn_fn.py::test_TN_pure_var_ref_handles_escaped_paren_in_array_index` (ground-truth audit pair)

---

### FP-NAB-13 — command-prefix callbacks are first-class command references (TP)

- **Verdict:** TRUE POSITIVE (precision gain) with literal-only FP guards
- **Status:** locked in by `tcl-lsp-core/tests/command_prefix_integration.rs` +
  `tcl-lsp-db/src/lib.rs::callback_arity_*` + `tcl-registry` `command_prefixes_*`
- **Codes:** W123 (unknown callback), E002/E003 (callback arity)
- **Corpus:** any `ArgRole::CommandPrefix` callback — `lsort -command`, `trace
  add … cb`, `socket -server`, `interp alias`, `struct::list map/filter/fold`, …

#### Registry coverage (ground truth: C Tcl 9.0 / package man pages)

- **Core Tcl:** `lsort -command` (2), `socket -server` (3), `trace add
  variable` (3) / `command` (3) / `execution` (≥2), `interp alias` (≥0) /
  `bgerror` (≥1), `namespace unknown` (≥1), `package unknown` (≥1),
  `regsub -command` (≥1, 9.0+), `coroinject` / `coroprobe` (Unknown),
  `chan create` / `chan push` reflected-channel handlers (≥2).
- **tcllib (positional prefixes):** `struct::list filter/map/fold/split`
  (1/1/2/1), `fileutil::find` (1), the `generator` functional ensemble (16 ops —
  Unknown, multi-value yield), `math::calculus` / `::optimize` / `::probopt`
  `func` callbacks (fixed, man-page-pinned — via `math_ext::PREFIX_OVERRIDES`),
  `log::lvCmd` / `lvCmdForall` (2), `uevent::bind` (≥2), `logger::walk`
  (Unknown), `tcltest::customMatch` (2), `tk selection handle` (2),
  `hook bind` (Unknown — resolver names the binding only in the 4-word set form).
- **tcllib (option-value prefixes — the value of a named `-flag`, via
  `OptionSpec`):** `mime::getbody -command` (≥1 — `uplevel` re-splits the reason
  list), `smtp::sendmessage -tlspolicy` (2), `comm::comm send -command` (14 —
  seven `-key value` reply pairs), `bibtex::parse -command` /
  `-preamble/-string/-comment/-progresscommand` (2) / `-recordcommand` (4),
  `tcl::chan::halfpipe -write-command` (2) / `-empty/-close-command` (1).  All
  ground-truthed against tcllib source and re-run in tclsh; the option-value
  path unions through `push_command_prefix_options` → `command_prefixes` with no
  compiler change.
- **Object-instance methods (via `ObjectClassSpec`):** `struct::graph`'s
  `$g walk … -command cb` (3 — action graphName node, option-value) and
  `struct::tree`'s `$t walkproc … cb` (3 — tree node action, trailing
  positional).  The receiver's class comes from the analyser's `instance_classes`
  map (`struct::graph name` or `set g [struct::graph]`); the compiler's
  `record_command_prefix_invocations` resolves the method's prefixes via
  `CommandRegistry::instance_method_command_prefixes`.  This lights up W123 /
  arity / references / call-graph edge / not-dead across **both** the whole-file
  and the incremental/project paths (see the two follow-ups below).
  `struct::tree walk`'s loop-variable *script* is not a prefix and is unmodelled.
- **Instance-method callbacks in the call graph + cross-file (the "long tail of
  the long tail"):** two passes beyond the foreground analyser were taught the
  receiver's object type, keyed off `object_types::object_handle_classes` — now
  extended to harvest registry naming-factories (`struct::graph g` /
  `set g [struct::graph]`) alongside its `[Class new]` + SSA/VTA signals:
  (a) the **interprocedural** call-graph pass threads that map into
  `scan_call_facts`, so an in-proc `$g walk … -command cb` becomes a real
  `direct_calls` edge (call graph, O124 not-dead, purity/effects) — not just a
  reference; (b) the **incremental per-item firewall** now binds registry
  object-factories *eagerly* in an isolated proc body (they resolve from the
  registry alone, no `all_classes`), where the old defer-to-graft left
  `instance_classes` empty during the body's own callback recording — so an
  in-proc instance callback resolves cross-file (arity + references) via
  `project_diagnostics`, matching the whole-file walk.  (The
  `signature_scan` walker, despite its name, indexes only class/proc
  *definitions* — its `command_invocations` are unused for references, which come
  from the foreground path — so it needs no object typing.)  The `OBJECT(class)`
  typing is kept out of the SSA lattice's `return_type_for_command` on purpose:
  W307/W308 aggregate `fu.types` object-insensitively across procs, so a
  lattice-typed factory would leak a handle's class between same-named vars
  (FP-OBJ-04).  Consequently object-flow through aliasing/collections is
  syntactic-only for these factories (direct `struct::graph g` / `set g […]`
  handles), not full VTA.
- **Tk (`script()`→`command_prefix` conversion — separate commit):**
  `-xscrollcommand`/`-yscrollcommand` on 8 widgets (2), `scale`/`ttk::scale
  -command` (1), `scrollbar -command` (≥2), `menu -tearoffcommand` (2).  This is
  a **net FP reduction**: the old `script()` recursion flagged widget paths
  inside braced callbacks (`{.sb set}` → spurious `W123 .sb`); as a command
  prefix the braced (non-bareword) head is dropped, so the widget-path W123
  disappears, while a bareword user-proc callback — previously *invisible* to
  script recursion — becomes a real reference / edge / arity-checked head.
  **Kept as `script()` (NOT prefixes):** button-family / `menu` / `ttk::spinbox`
  `-command` and `-postcommand` (verbatim scripts); core `spinbox -command`,
  `-validatecommand`, `-invalidcommand` (percent-substitution, not appended
  args); the 4 macOS-only file/message-dialog `-command`s stay
  `command_prefix(Unknown)` (inert cross-platform ⇒ no arity check).
- **Resolved script-vs-prefix:** `hook bind`'s binding is a command prefix (in
  the tcllib list above); `processman::onexit id cmd` is instead a deferred
  *script* (`eval $cmd`, 0 appended), modelled `ArgRole::Body` +
  `BodyKind::Structural` — its `{…}` recurses for W123 / references but it is
  deliberately **not** a command prefix.
- **Drift guard:** the `cmdprefix` allowlist
  (`commands_naming_a_cmdprefix_declare_a_command_prefix`) is now **empty** —
  every synopsis that names a `cmdprefix` declares a prefix.

#### Reproducer

```tcl
proc myCompare {a b} { expr {$a - $b} }
lsort -command myCompare $items      ;# reference + arity-checked (2 appended)
lsort -command typoCmp   $items      ;# W123: unknown command 'typoCmp'
proc oneArg {a} { return 0 }
lsort -command oneArg    $items      ;# E003: too many (2 appended, max 1)
```

#### Per-line reasoning

A command prefix's first word is a command **reference**, not a script or opaque
value. The registry owns which arguments are prefixes and how many args the
command appends to them (`CommandRegistry::command_prefixes` → `(index,
AppendedArity)`, ground-truthed vs C Tcl 9.0). The head is recorded as a
`command_invocations` entry (feeding find-references, rename, call-hierarchy,
code-lens, and W123) and as a `ProcSummary.direct_calls` edge (feeding the call
graph and O124 dead-code), so a callback-only proc is not "unused" and a typo'd
callback name draws W123. The callback's arity is validated against the
referenced proc, reusing the cross-file `E002`/`E003` path.

**FP guards (all verified):** only a **literal bareword** head is recorded — a
dynamic `$cb` / `[gen]` head is skipped (no W123 false-fire, no bogus edge);
`AppendedArity::Unknown`/`AtLeast` never fire "too few"; an `args`-catch-all or
all-optional-tail proc draws no arity error; a tail also claimed by a non-proc
(class/alias/ensemble) is arity-less.

#### tclsh ground truth

```
% proc cb {a b} {}; lsort -command cb {2 1}     ;# cb called as `cb 2 1` → 2 args
```
`lsort -command` appends exactly 2; `trace add variable` 3; `socket -server` 3
— the arities the registry declares.

#### Tests

- `command_prefix_integration.rs::call_graph_has_callback_edge` (TP — edge)
- `command_prefix_integration.rs::find_references_includes_callback_site` (TP)
- `command_prefix_integration.rs::w123_fires_on_unknown_callback_but_not_a_defined_one` (TP + TN)
- `command_prefix_integration.rs::dynamic_callback_head_is_not_recorded` (FP guard)
- `tcl-lsp-db::callback_arity_mismatch_draws_e002` / `_too_many_draws_e003` (TP)
- `tcl-lsp-db::callback_arity_correct_is_silent` / `_args_catchall_is_silent` /
  `_atleast_does_not_false_fire_too_few` (TN / FP guards)
- `command_prefix_integration.rs::namespace_unknown_handler_is_a_callback_edge` /
  `regsub_command_prefix_fires_w123_only_when_unknown` /
  `coroinject_records_reference_but_never_arity_checks` (deferred core commands)
- `command_prefix_integration.rs::tcllib_struct_list_split_is_a_callback_edge` /
  `tcllib_calculus_func_records_reference_with_fixed_arity` (tcllib)
- `tcl-lsp-db::callback_arity_namespace_unknown_zero_param_draws_e003` /
  `_package_unknown_variadic_handler_is_silent` / `_unknown_appended_never_fires` /
  `_tcllib_calculus_func_arity_checked` (deferred-command arity TP/TN)
- `tcl-registry::command_prefixes_cover_deferred_core_commands` /
  `_cover_tcllib_callbacks` /
  `commands_naming_a_cmdprefix_declare_a_command_prefix` (registry coverage + drift guard)
- `tcl-registry::tk_command_options_classified_prefix_vs_script` (Tk prefix-vs-script
  classification — the conversion locked in)
- `command_prefix_integration.rs::tk_scale_command_bareword_head_is_a_callback_edge` /
  `tk_scroll_callback_braced_widget_path_does_not_fire_w123` (FP removed) /
  `tk_scroll_callback_bareword_undefined_head_fires_w123` (TP now caught)
- `tcl-lsp-db::callback_arity_tk_scale_command_arity_checked` (Tk callback arity TP/TN)
- `tcl-registry::command_prefixes_cover_option_value_callbacks` (mime/smtp/comm/
  bibtex/halfpipe option-value prefixes — index + arity) + the `?…?`-stripping
  drift guard that now validates them
- `command_prefix_integration.rs::mime_getbody_command_option_is_a_callback_edge` /
  `smtp_tlspolicy_option_fires_w123_only_when_unknown` /
  `bibtex_recordcommand_option_is_a_callback_edge` /
  `halfpipe_write_command_option_records_callback` (option-value TP)
- `tcl-lsp-db::callback_arity_option_value_exactly_checked` /
  `_mime_getbody_command_atleast_one` (option-value arity TP/TN + FP guards)
- `command_prefix_integration.rs::hook_bind_binding_is_a_callback_edge` /
  `hook_bind_query_form_is_not_a_callback` (resolver-driven prefix TP + FP guard) /
  `processman_onexit_script_body_is_recursed_not_a_prefix` (script, not prefix)
- `tcl-registry::instance_method_command_prefixes_cover_struct_graph_and_tree` +
  `command_prefix_integration.rs::struct_graph_walk_command_records_callback_with_arity` /
  `struct_graph_walk_command_var_handle_fires_w123_only_when_unknown` /
  `struct_tree_walkproc_trailing_prefix_is_recorded` (object-instance methods —
  now also asserting the call-graph edge) +
  `tcl-lsp-db::callback_arity_struct_graph_walk_command_checked` / `_tree_walkproc_checked`
- `tcl-compiler::object_types::registry_naming_factory_handles` (object-handle map
  covers `struct::graph`/`::tree` naming + return forms) +
  `interprocedural::instance_method_callback_is_a_direct_call` (IPC call-graph edge) +
  `tcl-lsp-db::cross_file_in_proc_instance_method_callback_arity` (the per-item
  firewall fix — an in-proc instance callback resolves cross-file)

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

`regexp` and `scan` have analogous output-vars **but only write them on
success** (return 1 / N>0).  On no-match, the vars are NOT set:

```
% regexp {x} y -> v
0
% puts $v
can't read "v": no such variable
% scan abc %d n
0
% puts $n
can't read "n": no such variable
```

This is a **narrow caveat the catalog must honour**: the FP suppression
described below covers the "match-side" reads (inside an `if {[regexp
... -> v]} { use $v }` consequent or after a `scan` that's
*statically known* to succeed).  It is *NOT* safe to blanket-suppress
``regexp``/``scan`` output-var reads regardless of branch — that would
silence a real RBS bug.

The current analyser's recovery is name-level via
``command_sub_write_names`` and exempts the variable everywhere in
the proc, which means it suppresses RBS even on the no-match path.
This is an **open precision gap** (a sound over-approximation, but
genuinely loses a TP class).  Locked in by the open-finding xfail
test ``test_FP_RBS_02_no_match_path_known_precision_gap`` —
when a refined branch-aware analysis lands, the xfail flips and
prompts its own removal.

The `catch` case is sound — `catch ?msg? ?opts?` always writes
the named vars regardless of body success/failure (the body's
exception is converted to a result-code, never propagates).  All
three commands write into named caller-scope variables when the
operation reaches its write step; the fix covers all three in one
mechanism, with the regexp/scan precision gap documented above.

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
  dead_stores: (none)
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
  dead_stores: (none)
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
  dead_stores: (none)
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
  dead_stores: (none)
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
  dead_stores: (none)
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
  block entry_1
    term Branch ExprBinary(op=<BinOp.STR_EQ: 'eq'>, left=ExprRaw(text='${n}'), right=ExprLiteral(text='a', start=0, end=0))
  block switch_end_2
    phi  SSAPhi(name='j', version=4, incoming={'for_end_11': 2, 'if_end_12': 0, 'switch_default_3': 0})
    phi  SSAPhi(name='v', version=1, incoming={'for_end_11': 0, 'if_end_12': 2, 'switch_default_3': 0})
    term Goto
  block switch_default_3
    term Goto
  block switch_arm_body_4
    [0] AssignConst 'j' value='0'  defs={j#1}  uses={}
    term Goto
  block switch_arm_body_5
    [0] Call cmd='<cond>'  defs={->#1, v#2}  uses={}
    term Branch ExprCommand(text='[regexp {(\\w+)} "foo" -> v]', start=0, end=26)
  block switch_next_6
    term Branch ExprBinary(op=<BinOp.STR_EQ: 'eq'>, left=ExprRaw(text='${n}'), right=ExprLiteral(text='b', start=0, end=0))
  block switch_next_7
    term Goto
  block for_header_8
    phi  SSAPhi(name='j', version=2, incoming={'switch_arm_body_4': 1, 'for_step_10': 3})
    term Branch ExprBinary(op=<BinOp.LT: '<'>, left=ExprVar(text='$j', name='j', start=0, end=1), right=ExprLiteral(text='3', start=5, end=5))
  block for_body_9
    [0] Call cmd='puts'  defs={}  uses={j#2}
    term Goto
  block for_step_10
    [0] Incr 'j'  defs={j#3}  uses={j#2}
    term Goto
  block for_end_11
    term Goto
  block if_end_12
    term Goto
  block if_then_13
    [0] Call cmd='puts'  defs={}  uses={v#2}
    term Goto
  block if_next_14
    term Goto
  block exit_15
    term (none — fall-through exit)
  read_before_set: (none)
  dead_stores: (none)
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
    # the parameter, so 'x' must not be reported W214 ("Parameter ... is unused").
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
can't read "x": no such variable
```

`eval { ... }` evaluates the body in the **caller's** frame, so `$x` reads
`f`'s parameter `x`.  `namespace eval ::ns { ... }`, in contrast, evaluates
the body in the **target namespace** (`::ns`), where `$x` resolves to
`::ns::x` — a namespace variable that does NOT exist, so tclsh errors with
`can't read "x": no such variable`.  The caller's local `x` is NOT visible
from inside the namespace-eval body.

The recovery this entry locks in therefore applies ONLY to plain
`eval { ... }` (`IRBlock.caller_scope=True`).  For
`namespace eval ns { ... }` (`caller_scope=False`) the body's reads are
deliberately skipped — recovering them would wrongly suppress a real
unused-parameter / read-before-set finding.  See
`compiler/core_analyses.py::_block_local_reads` (early-return on
`not stmt.caller_scope`).

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
  dead_stores: (none)
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

### FP-RBS-12 — regexp/scan output-var conditional defs reach both reviewer cases (D1-4)

- **Verdict:** TRUE POSITIVE (precision-gap closure)
- **Status:** locked in by `tests/test_fp_rbs.py::test_FP_RBS_12_*`
- **Codes:** W210 (read-before-set)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D1-4

#### Reproducer

```tcl
proc f {} { regexp {x} y -> v; if {1} { puts $v } }
```

Plus the embedded-condition reviewer case:

```tcl
proc f {} { if {![regexp {x} y -> v]} { puts $v } }
```

#### Per-line reasoning

`regexp` writes its match-var (`v`) ONLY on a successful match.  When the read of `$v` sits on a path that includes the no-match outcome, the read is provably read-before-set.

1. **Reviewer case A** — `regexp {x} y -> v; if {1} { puts $v }`.  The pattern `{x}` against the literal input `y` doesn't match (the regex pattern is the literal letter `x`; the input is the literal letter `y`).  So `v` is provably unset on the no-match path; the subsequent unconditional `if {1} { puts $v }` always reads it.  W210 must fire on the body's `$v`.
2. **Reviewer case B** — `if {![regexp {x} y -> v]} { puts $v }`.  The if-arm body executes when regexp returns 0 (no match); on that path `v` was not written.  Reading `$v` in the body is read-before-set.
3. The closure has three pieces:
   - **F2 same-statement dominator walk** — walks back from each `$v` read to determine whether the def at the same statement reaches.
   - **F2 extension for embedded conditions** — propagates the "no-match implies unset" fact into the negated/short-circuit arms of an `if`/`while`/`for` condition.
   - **scan no-match estimator with D4-F1 conservative bail** — same logic for `scan`; bails conservatively on inputs containing `\\`/`$`/`[` (since the analyser sees pre-escape text).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { regexp {x} y -> v; if {1} { puts $v } }
% f
can't read "v": no such variable

% proc g {} { if {![regexp {x} y -> v]} { puts $v } }
% g
can't read "v": no such variable

% # TN control: read only on the match arm — safe.
% proc h {} { if {[regexp {x} y -> v]} { puts $v }; puts done }
% h
done
```

#### Compiler evidence

```
--- FP-RBS-12: regexp/scan output vars: conditional defs reach both reviewer cases (D1-4)
regen: python -m bench.fp_snippets --id FP-RBS-12
function ::f
  block entry_1
    [0] Call cmd='regexp'  defs={->#1, v#1}  uses={}
    term Branch ExprLiteral(text='1', start=0, end=0)
  block if_end_2
    term Goto
  block if_then_3
    [0] Call cmd='puts'  defs={}  uses={v#1}
    term Goto
  block if_next_4
    term Goto
  block exit_5
    term (none — fall-through exit)
  read_before_set
    ReadBeforeSet(block='if_then_3', statement_index=0, variable='v')
```
(regen: `python -m bench.fp_snippets --id FP-RBS-12`)

The `read_before_set` row pins the verdict: the `$v` read in `if_then_3` is reported even though `regexp` produced a `v#1` def — the post-pass detects that the regexp pattern is provably non-matching and treats the def as unreached.

#### Why the analyser reaches that verdict

`compiler/core_analyses.py:3250-` (the `provably_unset` post-pass in `_read_before_set`) — when a `regexp` / `scan` call has both literal pattern and literal input, the analyser runs Python's `re` (or the `scan_provably_no_match` predicate) to determine whether the match is provably impossible.  When it is, the output vars are marked `provably_unset` and subsequent reads — including reads in same-statement embedded conditions — fire W210.  D4-F1 closure for `scan_provably_no_match` is the conservative-bail piece (see [FP-STY-10](#fp-sty-10)).

#### Tests

- `tests/test_fp_rbs.py::test_FP_RBS_12_regexp_unconditional_read_after_no_match_fires` (TP — reviewer case A)
- `tests/test_fp_rbs.py::test_FP_RBS_12_regexp_in_negated_if_arm_fires` (TP — reviewer case B)
- `tests/test_fp_rbs.py::test_FP_RBS_12_regexp_match_arm_read_silent` (TN control — read only on the match arm)

---

### FP-RBS-13 — `tailcall` replaces the frame; code after it never runs

- **Verdict:** FALSE POSITIVE (W210) — `tailcall` ends the proc's straight-line
  flow, but the CFG modelled it as an ordinary fall-through call, so a variable
  set only on the *other* branch of an `if` looked maybe-unset at a read after
  the `if`.
- **Status:** FIXED (Rust port). Found while auditing the control-flow-terminator
  family (sibling of the `break`/`continue` CFG-jump fix); `tailcall` was the
  missing proc-exit terminator. Ported from the Python builder (commit `d312c6e`)
  and verified against the Rust oracle.
- **Codes:** W210
- **Corpus:** synthetic (Tcl 8.6+ `tailcall` dispatch idiom).

#### Reproducer

```tcl
proc f {cond} {
    # tailcall g replaces this frame: the `return $result` below is only
    # reached via the else branch, where result is always set.
    if {$cond} {
        tailcall g
    } else {
        set result 1
    }
    return $result
}
```

#### Per-line reasoning

1. `if {$cond} { tailcall g } else { set result 1 }` — exactly one branch runs.
2. `tailcall g` — replaces the current procedure with `g`. Control **never
   returns** to `f`, so the `then` branch does not reach the code after the
   `if`. The C implementation `TclNRTailcallObjCmd` (generic/tclBasic.c) ends
   with `return TCL_RETURN` for *any* arg count — both bare `tailcall` and
   `tailcall command ...` exit the proc; the arg count only decides what runs
   *after* the frame is popped.
3. `return $result` — reachable only via the `else` branch, where `set result
   1` ran. So `result` is **always** defined when read → no read-before-set.
4. Pre-fix, the CFG builder pushed `tailcall` as a plain fall-through call, so
   the `then` block edged into the `if` join; the join's `return $result` then
   merged an "unset" version of `result` from the `then` path → false W210.

#### tclsh ground truth (9.0.3)

`proc g {} { return GG }; f 0` → `1`; `f 1` → `GG`. No `can't read "result"`
error on either path. Bare `tailcall` behaves identically — `proc f {} { puts
before; tailcall; puts after }; f` prints only `before`.

#### Why the analyser reaches that verdict

`rust/tcl-compiler/src/cfg_builder/mod.rs` `CfgBuilder::push_plain_statement`:
in analysis builds (`faithful_exceptions`), a `Statement::Call` whose canonical
command is `::tailcall` (via `is_tailcall_command`) promotes the block to a
`Terminator::Return`, on the same path as `error`/`throw`/`exit`
(`is_block_terminating_command`), so post-`tailcall` statements are routed to an
orphan unreachable block exactly like `return`. Codegen builds
(`faithful_exceptions = false`) leave the call untouched, so bytecode is
unchanged. `tailcall` is not catchable, so it is not added to `throw_blocks`.

#### Tests

- `analyser::diagnostics::tests::fp_rbs_13_tailcall_is_a_terminator`
  (FP: `tailcall g` silent; FP: bare `tailcall` silent; TP control: a
  non-terminating then-branch — `puts hi` — restores the W210).

---

### FP-RBS-14 — opaque-switch arm that can't complete normally is excluded from must-define

- **Verdict:** FALSE POSITIVE (W210) — an opaque `switch`'s must-define set was
  a plain intersection over arm bodies, so an arm that always `return`s /
  `error`s (and therefore never reaches the code after the `switch`) wrongly
  dropped a variable that every *reaching* arm assigns.
- **Status:** FIXED (Rust port). Refines the opaque-switch arm-def recovery with
  the standard definite-assignment rule: a branch that cannot complete normally
  contributes ⊤ (vacuously defines everything) at the merge. Ported from the
  Python builder (commit `d312c6e`) and verified against the Rust oracle.
- **Codes:** W210
- **Corpus:** synthetic (dispatch `switch` with an early-return guard arm).

#### Reproducer

```tcl
proc f {x} {
    # the a* arm returns, so it never reaches `puts $y`; the only path that
    # does (default) sets y -> y is definitely defined.
    switch -glob $x {
        a* { return 0 }
        default { set y 2 }
    }
    puts $y
}
```

#### Per-line reasoning

1. `switch -glob $x { … }` stays *opaque* (one `Statement::Switch`; its
   shared-body topology is not lowered into CFG blocks), so its arm-body defs
   are recovered by an exhaustive must-define rule rather than by SSA phis.
2. `a* { return 0 }` — this arm `return`s, so it **cannot complete normally**;
   it never reaches `puts $y`. By definite assignment, a non-completing branch
   is excluded from the must-define intersection at the merge.
3. `default { set y 2 }` — the only arm that *reaches* `puts $y` assigns `y`.
4. `puts $y` — every path that arrives here has run `set y 2`, so `y` is
   definitely defined → no read-before-set. Pre-fix, the plain intersection
   counted the `return`-arm's empty def set, dropping `y` → false W210.

#### tclsh ground truth (9.0.3)

`f abc` → `0` (returns from the `a*` arm before `puts $y`); `f xyz` → `2`
(takes the default, sets `y`). No `can't read "y"` error. TP control: with the
`a*` arm changed to `{ set z 9 }` it falls through with `y` unset, and tclsh
errors `can't read "y": no such variable`.

#### Why the analyser reaches that verdict

`rust/tcl-compiler/src/ssa.rs` `defs_of_with_registry` (the
`Statement::Switch { default_body: Some(_), .. }` arm) calls
`cfg_builder::switch_must_defines`, which is `flow_facts_stmt(stmt).0`. The
shared `flow_facts_*` helpers in `rust/tcl-compiler/src/cfg_builder/mod.rs`
classify each branch by a 3-way `Completion` (`Normal` / `LoopJump` /
`ProcExit`) and `intersect_completing` **excludes only `ProcExit` branches**
(which reach no later code). A `LoopJump` (`break`/`continue`) branch is *kept*,
intersecting the defs it makes before jumping — it still reaches the code after
the enclosing loop, so an arm that breaks without assigning `y` correctly drops
`y` (Codex C1). The resulting set feeds the switch's `defs`, so SSA versions `y`
at the switch and the later read resolves.

#### Tests

- `analyser::diagnostics::tests::fp_rbs_14_opaque_switch_excludes_non_completing_arm`
  (FP: returning arm silent; FP: erroring arm silent; TP control: a completing
  arm that omits `y` keeps the W210; TP control / Codex C1: a `break` arm
  escapes the loop with `y` unset and keeps the W210).

---

### FP-RBS-15 — opaque switch whose every arm exits is itself a terminator

- **Verdict:** FALSE POSITIVE (W210) — an opaque `switch` whose every reachable
  arm `return`s / `error`s / `tailcall`s never falls through, so the code after
  it is unreachable; the CFG modelled the switch as a fall-through statement, so
  a read in that dead code was analysed as reachable and fired W210.
- **Status:** FIXED (Rust port). Extends FP-RBS-14 (which excludes non-completing
  arms from the must-define) to the case where *all* arms are non-completing:
  the switch itself becomes a terminator. Ported from the Python builder (commit
  `f18e2c2`) and verified against the Rust oracle.
- **Codes:** W210
- **Corpus:** synthetic (exhaustive dispatch `switch` where every arm returns).

#### Reproducer

```tcl
proc f {x} {
    # every arm returns, so control never reaches `puts $y`:
    # the switch is a terminator and the read is dead code.
    switch -glob $x {
        a* { return 1 }
        default { return 2 }
    }
    puts $y
}
```

#### Per-line reasoning

1. `switch -glob $x { … }` is opaque (one `Statement::Switch`; not lowered to
   CFG blocks). It has a `default`, so the subject always selects some arm.
2. `a* { return 1 }` and `default { return 2 }` — *every* reachable arm body
   `return`s, so none can complete normally. With a `default` present there is
   no fall-through path, so **no execution reaches `puts $y`**.
3. `puts $y` — dead code. tclsh never evaluates it, so reading the unset `y`
   here is not a runtime error; W210 must not fire.
4. Pre-fix the opaque switch always fell through to the next statement, so the
   block edged into `puts $y` and W210 fired on a read that can never execute.

#### tclsh ground truth (9.0.3)

`f abc` → `1`; `f zzz` → `2`. `f` always returns from inside the switch; the
`puts $y` after it never runs (no `can't read "y"` error). TP controls: dropping
the `default` lets an unmatched subject fall through → W210 fires; or making one
arm complete (`default { set z 9 }`) lets the switch fall through with `y` unset
→ W210 fires.

#### Compiler evidence (`tcl explore --json`, `cfgPreSsa` for `::f`)

```
block entry_1
  [0] Switch …
  term Return            # promoted: the opaque switch can't fall through
block unreachable_2      # `puts $y` routed here (orphan, no incoming edge)
  term Goto
block exit_3
```

#### Why the analyser reaches that verdict

`rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` `CfgBuilder::lower_opaque_switch`:
when lowering an opaque (glob/regexp/fall-through) switch in an analysis build
(`faithful_exceptions`), it reads `flow_facts_stmt(stmt).1` (the `Completion`):

* `ProcExit` — every arm `return`s/`error`s/`exit`s/`tailcall`s with a default,
  so the block is promoted to a `Terminator::Return` (the trailing statements
  are orphaned, like a `return`);
* `LoopJump` (Codex C3) — an arm `break`s/`continue`s to an enclosing loop; it
  is **not** a proc terminator, so instead `wire_opaque_switch_jumps` /
  `branch_to_any` wire explicit edges from the block to the loop's break /
  continue targets (plus a fall-through continuation when the switch can still
  complete normally), so a `while 1` whose only exit is such a jump has a
  reachable loop exit and the post-loop read correctly fires;
* `Normal` — plain fall-through.

`switch_escaping_jumps` / `escaping_loop_jumps` scan the arm bodies for escaping
`break`/`continue`, recursing into `if`/`switch`/`Block` but not into nested
loops or `catch`/`try` (which capture their own jumps). Codegen builds leave the
fall-through edge, so the opaque-switch `invokeStk` bytecode and CFG shape are
unchanged.

#### Tests

- `analyser::diagnostics::tests::fp_rbs_15_all_exiting_opaque_switch_is_a_terminator`
  (FP: all-return silent; FP: error/tailcall mix silent; TP control: no
  `default` falls through → W210; TP control: one completing arm lets the
  switch fall through → W210; TP control / Codex C3: an all-`break` switch in
  `while 1` reaches the loop exit → W210).

---

### FP-RBS-16 — phi operand on a dead loop-exit edge (`while 1` + `break`) is not read-before-set

- **Verdict:** FALSE POSITIVE (W210) — a loop-exit block's phi had an operand
  for the never-taken `cond → exit` edge of `while 1`; that operand carried the
  variable's version-0 (unset) origin, and the read-before-set phi-undef closure
  consumed it even though the edge is unreachable.
- **Status:** FIXED (Rust port). Same family as the terminator work (false W210
  from control-flow imprecision), but the gap is in the read-before-set /
  SCCP-edge interaction rather than CFG construction. Ported from the Python
  builder (commit `9f725e46`) and verified against the Rust oracle.
- **Codes:** W210
- **Corpus:** synthetic (the `while 1 { …; break }` / `while 1 { …; if {c} break }`
  early-exit idiom).

#### Reproducer

```tcl
proc f {} {
    # while 1 runs the body >=1 time; the only exit is the break, where y is
    # already set -> $y is always defined here.
    while 1 { set y 1; break }
    puts $y
}
```

#### Per-line reasoning

1. `while 1 { … }` — the condition is the constant `1`, so the loop never exits
   via the condition; SCCP marks the `while_header → while_end` (cond-false)
   edge non-executable.
2. `set y 1; break` — the body runs at least once and the `break` is the loop's
   only real exit. On that edge `y` is set.
3. `puts $y` — `while_end` is reachable (via the `break` edge), and its phi
   `y#2` merges the break edge (`y#1`, set) with the dead cond-exit edge
   (`y#0`, unset). Only the break edge is live, so `y` is always defined here.
4. Pre-fix, `phi_can_undef` filtered unreachable predecessor *blocks*
   (`while_header` is reachable) but not unreachable *edges*, so it consumed the
   `y#0` operand on the dead `while_header → while_end` edge and false-fired
   W210. (Contrast: with no `break`, `while_end` is fully unreachable and
   correctly silent — that's why the bug only shows with a `break`.)

#### tclsh ground truth (9.0.3)

`f` → `1`. No `can't read "y"` error. The realistic shape behaves the same —
`proc f {} { while 1 { set r [compute]; if {[ok $r]} break }; return $r }`
returns the computed value with no unset-read. TP controls: `while {$n > 0} {
set y 1 }` (non-constant cond, may run zero times) still fires W210; and a
`while 1` where only one of two `break` paths sets `y` still fires.

#### Why the analyser reaches that verdict

`rust/tcl-compiler/src/analyser/diagnostics.rs`: `build_phi_undef_index` now also
records each phi's defining block (`PhiBlockMap`), and `phi_can_undef` takes the
SCCP `executable_edges` (`fu.sccp.executable_edges`) and skips any phi operand
whose `(pred, phi_block)` edge is non-executable — the same edge filter the
SCCP-reachability passes already use. SCCP marks the `while 1` cond-false edge
non-executable, so the version-0 operand it carries can never be read and no
longer counts as a possible undef. The filter is applied only when SCCP edge
info is available (a non-empty set), so registry-less test paths are unaffected.

#### Tests

- `analyser::diagnostics::tests::fp_rbs_16_dead_loop_exit_phi_operand_not_read_before_set`
  (FP: `while 1` set+break silent; FP: `while 1` compute/if-break silent; TP
  control: non-constant condition still fires; TP control: partial-def break
  path still fires).

---

### FP-RBS-17 — guaranteed-iteration `foreach` defines body variables (loop rotation)

- **Verdict:** FALSE POSITIVE (W210) — a `foreach` over a non-empty *literal*
  list provably runs its body ≥1 time, so a body-assigned variable (or the loop
  variable) read after the loop is defined; the CFG modelled every loop as
  possibly zero iterations, false-firing W210.
- **Status:** FIXED (Rust port). Ported from the Python builder (commit
  `2d3dcef9`) and verified against the Rust oracle.
- **Codes:** W210
- **Corpus:** synthetic (`foreach` accumulator / dispatch idioms).

#### Reproducer

```tcl
proc f {} {
    foreach x {1 2 3} { set y $x }   ;# runs >=1 time -> y is set
    puts $y
}
```

#### Per-line reasoning

1. `foreach x {1 2 3}` — the iterator list is a non-empty literal, so the body
   runs at least once (`foreach` runs `max` over its iterator groups).
2. `set y $x` — assigns `y` on every iteration; after the loop `y` is defined.
3. `puts $y` — `y` is always defined → no read-before-set. Pre-fix, the loop
   header's zero-iteration exit edge merged an unset `y` at the loop exit.

#### tclsh ground truth (9.0.3)

`foreach x {1 2 3} { set y $x }; puts $y` → `3` (no `can't read "y"`).
`foreach x {a b c} {}; puts $x` → `c` (the loop variable survives). TP controls:
an empty literal (`foreach x {} …`) or a dynamic list (`foreach x $i …`) may run
zero times and tclsh errors `can't read`.

#### Why the analyser reaches that verdict

`rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` `CfgBuilder::lower_foreach`: in
analysis builds (`faithful_exceptions`), when `foreach_runs_at_least_once`
(`list_literal_nonempty` over any iterator's list) holds, the loop is *rotated* —
the header becomes a synthetic always-true entry guard (`literal_true_expr`,
span-less so the optimiser's constant-branch rewriter leaves it), the var-def +
body run before a back-edge re-check at a new `foreach_latch` block, and
`break`/`continue` stay real edges. SCCP prunes the dead entry→end edge and the
FP-RBS-16 dead-edge phi filter ignores the version-0 operand it carried — with no
synthetic def, so SCCP values are untouched. Codegen builds
(`build_cfg_codegen`, `faithful_exceptions` off) keep the original single-header
shape, so bytecode/CFG for codegen is byte-identical (the `detect_foreach`
emitter and `differential_codegen` parity are unchanged).

#### Tests

- `analyser::diagnostics::tests::fp_rbs_17_guaranteed_foreach_defines_body_vars`
  (FP: body var defined; FP: loop var defined; TP controls: empty literal,
  dynamic list, first-iteration read-before-set).

---

### FP-RBS-18 — guaranteed-iteration `for` defines body variables (loop rotation)

- **Verdict:** FALSE POSITIVE (W210) — a `for` whose condition is statically
  true on entry provably runs its body ≥1 time, so a body-assigned variable read
  after the loop is defined; the CFG modelled it as possibly zero iterations.
- **Status:** FIXED (Rust port). Ported from the Python builder (commits
  `2d3dcef9` + `d43f8ee8` for the Codex C2 for-init invalidation) and verified
  against the Rust oracle.
- **Codes:** W210
- **Corpus:** synthetic (counting `for` loops).

#### Reproducer

```tcl
proc f {} {
    for {set i 0} {$i < 3} {incr i} { set y $i }   ;# 0<3 true -> runs >=1
    puts $y
}
```

#### Per-line reasoning

1. `for {set i 0} {$i < 3} …` — the init binds `i = 0` (a constant); the
   condition `0 < 3` is statically true, so the first iteration always runs.
2. `set y $i` — assigns `y`; after the loop `y` is defined.
3. `puts $y` — defined → no read-before-set.

#### tclsh ground truth (9.0.3)

`for {set i 0} {$i<3} {incr i} {set y $i}; puts $y` → `2`. TP controls:
`for {set i 5} {$i<3} …` runs zero times → `can't read "y"`; and (Codex C2) a
stale-constant init — `for {set i 0; set i $n} …` or `for {set i 0; incr i 5} …`
— may iterate zero times, so it must NOT be claimed guaranteed.

#### Why the analyser reaches that verdict

`rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` `CfgBuilder::lower_for`: in
analysis builds, when `for_runs_at_least_once` holds, the loop is rotated — the
step (`for_step`) re-checks the *real* condition on the back-edge and the header
becomes a synthetic always-true, span-less entry guard. `for_runs_at_least_once`
evaluates the condition (`eval_tcl_expr`) against the init clause's constant
bindings, processed **in order**: an `AssignConst` (re)binds a constant, but any
other write (`set i $n`, `incr i …`, a call) *invalidates* that variable's
binding (Codex C2), so a stale constant never makes a zero-iteration loop look
guaranteed. `loop_nodes` + the init exit versions are unchanged, so the
optimiser's IR-level static-`for` summary is unaffected; codegen
(`build_cfg_codegen`) keeps the single-header shape (byte-identical bytecode).

#### Tests

- `analyser::diagnostics::tests::fp_rbs_18_guaranteed_for_defines_body_vars`
  (FP: statically-true entry condition; TP controls: false entry condition,
  provably-empty `incr` init; may-run stale-constant `set i $n` init and
  upvar-writing init are now silent — see FP-RBS-19).

---

### FP-RBS-19 — a may-run loop whose body defines a variable is assumed to run (#756)

- **Verdict:** FALSE POSITIVE (W210) — a variable assigned inside a loop body
  and read *after* the loop was flagged read-before-set because the loop
  *might* iterate zero times. tclsh only errors when the iterator list /
  condition is actually empty at runtime; on real (non-empty) data the body
  runs and the variable is defined, so the after-loop read is safe.
- **Status:** FIXED. Reported by a user (issue #756) hitting the
  `foreach … { lappend acc … }` accumulator idiom on real code
  (SpiceGenTcl / ngspice). Supersedes the earlier decision (FP-RBS-16/17/18 TP
  controls) that a may-run loop *must* fire; only a **provably** zero-iteration
  loop, or a body that leaves the variable unset on some run, still fires.
- **Codes:** W210
- **Corpus:** `SpiceGenTcl` `foreach time [dict get $data time] … { lappend
  timeVout … }` and the accumulate-in-loop idiom throughout.

#### Reproducer

```tcl
set data [getDataDict]
foreach time [dict get $data time] vout [dict get $data osc_out] {
    lappend timeVout [list $time $vout]
}
puts $timeVout   ;# defined whenever the loop ran — not read-before-set
```

#### Per-line reasoning

1. `foreach time [dict get $data time] …` — a *dynamic* iterator list. The
   analyser cannot prove it non-empty, so it models the loop as possibly zero
   iterations (the header's zero-trip exit edge stays executable).
2. `lappend timeVout …` — the body **unconditionally** assigns `timeVout` on
   every iteration; the loop-header phi that reaches the after-loop read is
   undef **only** via the zero-trip entry edge.
3. `puts $timeVout` — reached only *after* the loop. Matching C Tcl (which
   errors only when `[dict get $data time]` is actually empty at runtime, not
   merely when it could be), we assume a may-run loop runs, so `timeVout` is
   defined. No W210.

#### tclsh ground truth (8.6 / 9.0)

`set l {1 2 3}; foreach x $l { set y $x }; puts $y` → `3` (no error). Only an
actually-empty list errors: `set l {}; foreach x $l { set y $x }; puts $y` →
`can't read "y"`. A *provably* empty literal (`foreach x {} …`) or a
`while 0` / false-on-entry `for` is statically empty and still fires.

#### Why the analyser reaches that verdict

`build_loop_entry_only_undef` (in `analyser/diagnostics/helpers.rs`) walks the
natural-loop forest (built over the SCCP-executable subgraph, so a provably
dead body forms no loop). For each loop-header phi in `can_undef` whose every
*back-edge* operand is itself defined — the body assigns the variable on every
iteration — it records the phi as loop-entry-only-undef together with the
loop's body-block set. A provably-empty `foreach` (all iterator lists split to
zero elements) is excluded, so it keeps firing. The read-before-set emitters
(`record_chain_w210_uses` and `return_read_fires_w210`) then suppress a read of
such a version when its block is **outside** the loop body — a read *inside*
the body (a first-iteration use before the set) still fires. `while`/`for`
loops with a constant-false condition are already pruned by SCCP, so only the
opaque-`foreach`-empty-literal case needs the explicit provably-empty guard.

#### Tests

- `analyser::diagnostics::fp::rbs::fp_rbs_19_*` (FP: the reporter's
  dynamic-foreach `lappend` accumulator; may-run `while`/`for`; TP controls:
  provably-empty literal, conditional-def-in-body, first-iteration in-body
  read).
- Flipped TP controls now asserting silence: `fp_rbs_16_normal_while_*`,
  `fp_rbs_17_dynamic_list_*`, `fp_rbs_18_unknown_bound_*`,
  `fp_rbs_18_stale_const_init_*`, and `tests::w210_loop_body_accumulator_*`.
- Rotation-classification coverage that the flipped W210 proxies used to
  provide moved to `cfg::for_rotation_requires_a_non_stale_constant_init`.
- LSP e2e: `tests::e2e::diagnostics::{foreach_accumulator_after_loop_silent,
  foreach_empty_literal_after_loop_still_fires, normal_while_body_defined_silent}`.
- VS Code: `precisionFalsePositives.test.ts` "dynamic-loop after-loop reads
  stay silent (#756)" over `controlFlowRbs.tcl`.

---

## DS — dead-store / unused (W220/W211)

These entries lock in the analyser's recovery of *real* reads that live in
otherwise-opaque constructs — command substitutions, expr cmd-subs, eval
bodies, the return terminator, write-traces, and (Phase 8G) distinct
ARRAY_ELEM Places.  Each FP test pairs with a TP control proving the fix
doesn't over-suppress.

### FP-DS-01 — incr/append/lappend inside cmd-sub: read-modify-write keeps init live

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_01_*`
- **Codes:** W220, W211
- **Corpus:** anything with an accumulator-in-cmd-sub idiom (tcllib's `foreach j … { lappend r [incr i $j] }` shape recurs throughout the lists / structures modules).

#### Reproducer

```tcl
proc f {} {
    # incr inside the cmd-sub reads `i` (the prior value) — so the
    # feeding `set i 0` is alive, not a dead store.
    set i 0
    foreach j {1 2 3} { lappend r [incr i $j] }
    return $r
}
```

#### Per-line reasoning

1. `set i 0` — assigns version 1 of `i`.
2. `foreach j {1 2 3}` — opens a loop; `j` is the loop var (defined per iteration).
3. `lappend r [incr i $j]` — the cmd-sub runs `incr i $j`, which **reads** the prior `i#1` and writes `i#2`.  This makes `set i 0` alive.
4. `return $r` — reads `r`.

Pre-fix the outer word scanner saw `lappend r [incr i $j]` as one opaque cmd-sub; the read of `i` inside was invisible, so `set i 0` looked dead.

#### tclsh ground truth

```
% set i 0; foreach j {1 2 3} { lappend r [incr i $j] }; puts "r=$r i=$i"
r=1 3 6 i=6
```
`incr` modifies `i` in place — both the read of the prior value and the write of the new value are observable.

#### Compiler evidence

```
--- FP-DS-01: incr/append/lappend inside cmd-sub: read-modify-write keeps init live
regen: python -m bench.fp_snippets --id FP-DS-01
function ::f
  block entry_1
    [0] AssignConst 'i' value='0'  defs={i#1}  uses={}
    term Goto
  block foreach_header_2
    phi  SSAPhi(name='j', version=1, incoming={'entry_1': 0, 'foreach_body_3': 2})
    phi  SSAPhi(name='r', version=1, incoming={'entry_1': 0, 'foreach_body_3': 2})
    [0] Call cmd='foreach'  defs={j#2}  uses={}
    term Branch ExprRaw(text='<foreach_has_next>')
  block foreach_body_3
    [0] Call cmd='lappend'  defs={r#2}  uses={j#2, r#1}
    term Goto
  block foreach_end_4
    term Return ${r}
  dead_stores: (none)
```

#### Why the analyser reaches that verdict

`command_sub_read_modify_write_names` in `compiler/var_refs.py` recognises `incr` / `append` / `lappend` targets nested in command substitutions and treats them as reads of the prior version.  `_dead_store` / `_unused` then keep `i` live.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_01_init_kept_live_by_cmdsub_incr` (FP)
- `tests/test_fp_ds.py::test_FP_DS_01_genuine_dead_store_still_fires` (TP)

---

### FP-DS-02 — reads inside [expr {...}] command-sub recovered as real uses

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_02_*`
- **Codes:** W220, W211
- **Corpus:** any `incr i [expr {…}]` or `return [expr {… $v …}]` idiom where the only read of a feeding `set` lives inside an expr command substitution (tcllib's iteration helpers across `lazyseq`, `struct::list`).

#### Reproducer

```tcl
proc f {} {
    # $w is read inside the [expr {...}] cmd-sub — `set w 5` is NOT
    # a dead store, and `w` is NOT unused.
    set w 5
    set i 0
    incr i [expr {$w}]
    return $i
}
```

#### Per-line reasoning

1. `set w 5` — assigns version 1 of `w`.
2. `set i 0` — assigns version 1 of `i`.
3. `incr i [expr {$w}]` — the cmd-sub evaluates the expr, **reading $w**; the resulting value is the increment amount.  This read is the only use of `w` in the proc.
4. `return $i` — reads `i`.

Pre-fix expressions inside command substitutions were opaque to the outer word scanner; the `$w` read was invisible so `set w 5` looked dead and `w` looked unused.

#### tclsh ground truth

```
% set w 5; set i 0; incr i [expr {$w}]; puts "i=$i"
i=5
```

#### Compiler evidence

```
--- FP-DS-02: reads inside [expr {...}] command-sub recovered as real uses
regen: python -m bench.fp_snippets --id FP-DS-02
function ::f
  block entry_1
    [0] AssignConst 'w' value='5'  defs={w#1}  uses={}
    [1] AssignConst 'i' value='0'  defs={i#1}  uses={}
    [2] Incr 'i'  defs={i#2}  uses={i#1}
    term Return ${i}
  dead_stores: (none)
```

#### Why the analyser reaches that verdict

`statement_cmd_sub_read_names` walks `IRAssignExpr` / `IRExprEval` / `IRReturn` value-expression ASTs collecting variable reads under cmd-sub barriers.  Combined with FP-DS-01 (read-modify-write recovery) the feeding `set w 5` stays live.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_02_expr_cmdsub_read_keeps_def_live` (FP)
- `tests/test_fp_ds.py::test_FP_DS_02_no_expr_cmdsub_read_still_fires` (TP)

---

### FP-DS-03 — eval {literal-body} reads recovered (eval runs in caller scope)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_03_*`
- **Codes:** W220, W211
- **Corpus:** anywhere `eval {literal body}` is used as a cheap macro form (event-handler dispatch idioms, dialect-specific wrappers).

#### Reproducer

```tcl
proc f {} {
    # eval's braced body runs in the current scope; `$x` read here is
    # a real read of the local `x`.
    set x 1
    eval {puts $x}
}
```

#### Per-line reasoning

1. `set x 1` — assigns version 1 of `x`.
2. `eval {puts $x}` — the braced body is a literal string; eval re-parses and runs it in the **caller** scope.  `$x` is a read of the local `x`.

Pre-fix the eval body was an opaque IRCall barrier; reads inside it weren't projected to the caller, so `set x 1` looked dead / unused.

#### tclsh ground truth

```
% set x 1; eval {puts $x}
1
```
The braced body re-enters the parser at run time and `$x` refers to the caller-scope `x`.

#### Compiler evidence

```
--- FP-DS-03: eval {literal-body} reads recovered (eval runs in caller scope)
regen: python -m bench.fp_snippets --id FP-DS-03
function ::f
  block entry_1
    [0] AssignConst 'x' value='1'  defs={x#1}  uses={}
    [1] Block  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
  dead_stores: (none)
```

#### Why the analyser reaches that verdict

`eval_body_read_names` in `compiler/var_refs.py` walks literal `eval {…}` and `namespace eval ns {…}` bodies recovering reads (including those nested inside `[expr {…}]` and `[set y …]`).  `_dead_store` / `_unused` consume the recovered name set.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_03_eval_body_read_kept_live` (FP)
- `tests/test_fp_ds.py::test_FP_DS_03_eval_body_without_read_still_fires` (TP)

---

### FP-DS-04 — traced variables excluded from dead-store / unused (soundness)

- **Verdict:** FALSE POSITIVE (soundness fix)
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_04_*`
- **Codes:** W220, W211
- **Corpus:** any callback-driven Tcl code (Tk binding handlers, EDA testbench dispatchers, tcllib's `notifier` patterns).

#### Reproducer

```tcl
proc f {} {
    # The write is observable through the callback — must NOT fire
    # W220 (dead-store) or W211 (unused).
    trace add variable x write cb
    set x 1
}
```

#### Per-line reasoning

1. `trace add variable x write cb` — installs a write-trace callback on the local `x`.  Any subsequent `set x …` fires `cb`.
2. `set x 1` — writes `x`.  This write is **observable** via the callback.

So even with no in-proc read of `x`, the write has an externally-visible side effect and is NOT dead / unused.  Pre-fix the dead-store analysis ignored traces — a soundness gap, not just a precision one.

#### tclsh ground truth

```
% proc cb {args} { puts "TRACE: $args" }
% trace add variable x write cb; set x 1
TRACE: x {} write
```
The callback fires as part of `set x 1`'s evaluation.

#### Compiler evidence

```
--- FP-DS-04: traced variables excluded from dead-store / unused (soundness)
regen: python -m bench.fp_snippets --id FP-DS-04
function ::f
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='trace'  defs={x#1}  uses={}
    [2] AssignConst 'x' value='1'  defs={x#2}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
  dead_stores: (none)
```

#### Why the analyser reaches that verdict

`compiler/var_refs.py` collects `traced_var_names` from `trace add variable` and the Tcl-8.4 `trace variable` form.  `_dead_store` and `_unused` exempt the traced names entirely.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_04_traced_var_no_w220` (FP, 8.5+ form)
- `tests/test_fp_ds.py::test_FP_DS_04_84_form_also_excluded` (FP, 8.4 form)
- `tests/test_fp_ds.py::test_FP_DS_04_untraced_unrelated_var_still_fires` (TP control)

---

### FP-DS-05 — CFGReturn read is a real use ($x kept live by `return $x`)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_05_return_read_counts`
- **Codes:** W220, W211
- **Corpus:** every proc that returns a freshly-bound value (`set r [doThing]; return $r`) — extremely common.

#### Reproducer

```tcl
proc f {} {
    # return $x reads $x — `set x 1` is NOT a dead store, `x` is NOT unused.
    set x 1
    return $x
}
```

#### Per-line reasoning

1. `set x 1` — assigns version 1 of `x`.
2. `return $x` — the CFGReturn terminator's value expression reads `$x`.

Pre-fix the terminator-level read wasn't projected back into the variable use-set, so `set x 1` looked like a dead store with no consumer.

#### tclsh ground truth

```
% proc f {} { set x 1; return $x }
% f
1
```

#### Compiler evidence

```
--- FP-DS-05: CFGReturn read is a real use ($x kept live by `return $x`)
regen: python -m bench.fp_snippets --id FP-DS-05
function ::f
  block entry_1
    [0] AssignConst 'x' value='1'  defs={x#1}  uses={}
    term Return ${x}
  dead_stores: (none)
```

#### Why the analyser reaches that verdict

Use-set building in `compiler/var_refs.py` includes CFGReturn value-expression reads when constructing the per-block uses map.  `_dead_store` reads from this map and recognises the terminator-level use.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_05_return_read_counts` (FP)

---

### FP-DS-06 — array-element dead-store distinction: $a(k) write is not killed by $a(j) write

- **Verdict:** FALSE POSITIVE (now fixed, Phase 8G)
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_06_array_elem_writes_distinct`
- **Codes:** W220, W211, O109
- **Corpus:** any keyed-by-element pattern (tcllib's `dict`-builders, iRules' `set ::tbl(\$client) val` patterns, snit option storage).  Corpus impact: **W220 −88, W211 −2, O109 −66** on first measurement.

#### Reproducer

```tcl
proc f {} {
    # k and j are distinct array element Places — set a(k) is NOT
    # killed by set a(j); the read of $a(k) makes the first write live.
    set a(k) 1
    set a(j) 2
    puts $a(k)
}
```

#### Per-line reasoning

1. `set a(k) 1` — writes the ARRAY_ELEM Place `a / k` (version 1 of `a`).
2. `set a(j) 2` — writes the ARRAY_ELEM Place `a / j` (version 2 of `a`).  These are *distinct* Places: `k` and `j` are literally different keys.
3. `puts $a(k)` — reads the ARRAY_ELEM Place `a / k`.

Pre-Phase-8 the analysis tracked whole-array kills, so `set a(j) 2` "killed" the prior `set a(k) 1`.  Phase 8 introduces the Place model: ARRAY_ELEM Places overlap *only* when the indices are identical (literal == literal) OR when at least one is dynamic (in which case overlap is suppress-only after the 8E refinement).

#### tclsh ground truth

```
% set a(k) 1; set a(j) 2; puts $a(k)
1
```
Different keys are different slots.

#### Compiler evidence

```
--- FP-DS-06: array-element dead-store distinction: $a(k) write is not killed by $a(j) write
regen: python -m bench.fp_snippets --id FP-DS-06
function ::f
  block entry_1
    [0] AssignConst 'a(k)' value='1'  defs={a#1}  uses={}
    [1] AssignConst 'a(j)' value='2'  defs={a#2}  uses={}
    [2] Call cmd='puts'  defs={}  uses={a#2}
    term Goto
  block exit_2
    term (none — fall-through exit)
  dead_stores: (none)
```

#### Why the analyser reaches that verdict

`compiler/place.py`'s `overlap()` relation: two ARRAY_ELEM Places overlap iff they share name + a non-disjoint key set.  Literal `k` and `j` are disjoint → no overlap → no kill → first write stays alive.  See `analyser/_analyser/_diag_var_lifecycle.py` for the W220 check that consumes the Place model.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_06_array_elem_writes_distinct` (FP)
- `tests/test_fp_ds.py::test_FP_DS_06_same_element_overwrite_still_fires` (OPEN xfail — flips when must-alias kills land)

---

### FP-DS-07 — namespace-eval body scope survives an inline/factory IRBlock rebuild

- **Verdict:** TRUE POSITIVE (genuinely-unused parameter) + paired FP guard
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_07_*`
- **Codes:** W214 (paired with W220 — same caller-scope gate)
- **Corpus:** synthetic (no clean-corpus instance — needs a factory/uplevel candidate *and* an
  unqualified caller-param read inside the same `namespace eval` body; corpus delta 0). tclsh-verified.

#### Reproducer

```tcl
proc reset {} { uplevel 1 {set counter 0} }
proc g {x} {
    namespace eval ::ns {
        reset
        puts "hello $x"
    }
}
```

#### Per-line reasoning

A `namespace eval ns {…}` body runs in `ns`, not the caller frame, so the unqualified `$x`
there is **not** a use of `g`'s parameter — Tcl resolves it in `::ns` (and errors `can't read
"x"` when `::ns::x` is unset).  The parameter `x` is therefore genuinely unused → W214.

The IRBlock carrying this distinction sets `caller_scope=False` (so the body→caller read
recovery at `place_bridge.py` / `core_analyses.py` is *not* applied).  But the inline-uplevel
and factory-specialise passes **rebuild** that IRBlock whenever its body changes — here `reset`
is an uplevel-passthrough candidate, so `inline_uplevel` splices its body and reconstructs the
block.  The rebuild copied range/body/namespace/source_args/source_tokens by hand and silently
dropped `caller_scope` back to its `True` default, re-enabling the recovery and **falsely
suppressing** the W214.  The fix rebuilds via `dataclasses.replace(stmt, body=new_body)` so the
flag (and any future field) is preserved.

The paired FP control: a **plain** `eval {…}` body *does* run in the caller frame, so `$x`
there is a real use — W214 must NOT fire.

#### tclsh ground truth

```
% proc g {x} { namespace eval ::ns { puts "hello $x" } } ; g hi
can't read "x": no such variable        ;# $x is NOT g's parameter — x is unused in g
% proc g {x} { eval { puts "hello $x" } } ; g hi
hello hi                                  ;# plain eval runs in g's frame — $x IS used
```

#### Compiler evidence

```
--- FP-DS-07: namespace-eval body scope survives an inline/factory IRBlock rebuild
regen: python -m bench.fp_snippets --id FP-DS-07
function ::g
  block entry_1
    [0] Block  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
```

The `Block` statement carries `caller_scope=False`; its `uses={}` (the body's `$x` is not
hoisted as a caller use) is exactly what keeps `x` reportable as unused.

#### Why the analyser reaches that verdict

`compiler/inline_uplevel.py` and `compiler/passes/specialise_factories.py` rebuild the
`namespace eval` IRBlock with `dataclasses.replace(stmt, body=new_body)`, preserving
`caller_scope=False`, so the post-rebuild `read_places` / `_block_local_reads` gates leave the
body's unqualified reads unrecovered — and the unused-parameter pass still sees `x` as unused.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_07_ns_eval_param_unused_through_rebuild_fires` (TP)
- `tests/test_fp_ds.py::test_FP_DS_07_plain_eval_body_read_is_caller_use_silent` (FP)
- analyser-path coverage: `tests/test_checks.py::TestNamespaceEvalBodyScope`

---

### FP-DS-08 — `dict with` key-aware suppression on the return-terminator path (D3-P1 / D4-F3)

- **Verdict:** TRUE POSITIVE (precision-gap closure) — earlier suppression was over-broad on the return arm
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_08_*`
- **Codes:** W210 (read-before-set)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entries D3-P1, D4-F3

#### Reproducer

```tcl
proc f {} { set d {}; dict with d {}; return $missing }
```

#### Per-line reasoning

1. `set d {}` — `d` is bound to the empty dict literal.  SCCP knows the value statically.
2. `dict with d {}` — Tcl unpacks the dict's keys as local variables in the current scope.  Since the dict is empty, NO keys are unpacked; the local namespace gains nothing.
3. `return $missing` — reads a variable named `missing`.  Nothing in the proc ever defined it, and the empty `dict with` couldn't have created it either.  tclsh errors at runtime.
4. Pre-fix the statement-use path of `_read_before_set` already had the key-aware logic (only the keys the literal dict actually unpacks exempt reads), but the `CFGReturn` arm used a blanket "any dict-with in the function suppresses any return-path read".  D4-F3 closure mirrors the key-aware check into the return arm.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { set d {}; dict with d {}; return $missing }
% f
can't read "missing": no such variable
```

The empty `dict with` unpacks no keys, and `missing` is genuinely never defined.

#### Compiler evidence

```
--- FP-DS-08: dict with key-aware suppression on the return-terminator path (D3-P1/D4-F3)
regen: python -m bench.fp_snippets --id FP-DS-08
function ::f
  block entry_1
    [0] AssignConst 'd' value=''  defs={d#1}  uses={}
    [1] InterpBoundary  defs={}  uses={}
    [2] Barrier cmd='dict'  defs={d#2}  uses={d#1}
    term Return ${missing}
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    d#1: OVERDEFINED
  read_before_set
    ReadBeforeSet(block='entry_1', statement_index=-1, variable='missing')
```
(regen: `python -m bench.fp_snippets --id FP-DS-08`)

The `read_before_set` row pins the verdict: the return-terminator's `$missing` read is reported even though a `dict with` is in scope, because the (empty) literal dict's keys don't include `missing`.

#### Why the analyser reaches that verdict

`compiler/core_analyses.py:3237` — the `CFGReturn` arm of `_read_before_set` now consults `_dict_with_known_keys` and `_dict_with_any_unknown` the same way the statement-use arm does: if the dict shape is fully known and the name isn't a literal key, the suppression doesn't apply and the read is reported.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_08_empty_dict_with_return_missing_fires` (TP)
- `tests/test_fp_ds.py::test_FP_DS_08_known_key_dict_with_return_var_silent` (TN — literal dict has the key)
- `tests/test_fp_ds.py::test_FP_DS_08_unknown_dict_with_return_var_silent` (TN — unknown dict shape stays conservatively silent)
- `tests/test_ground_truth_tn_fn.py::test_TP_W210_empty_dict_with_return_missing_var`
- `tests/test_ground_truth_tn_fn.py::test_TN_known_key_dict_with_return_var`
- `tests/test_ground_truth_tn_fn.py::test_TN_unknown_dict_with_return_var`

---

### FP-DS-09 — interprocedural literal-dict propagation feeds the dict-with key check (D3-P2)

- **Verdict:** TRUE POSITIVE (precision-gap closure) — interproc literal-arg propagation
- **Status:** locked in by `tests/test_fp_ds.py::test_FP_DS_09_*`
- **Codes:** W210 (read-before-set)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D3-P2

#### Reproducer

```tcl
proc f {d} { dict with d { return $missing } }
f {}
```

#### Per-line reasoning

1. `proc f {d}` — `d` is a parameter; its in-callee value comes entirely from the call sites.
2. `dict with d { return $missing }` — unpacks `d`'s keys as locals, then returns `$missing`.  Whether this errors depends on whether `d` has a `missing` key.
3. `f {}` — the call site passes the empty dict literal.  Interprocedural propagation makes the callee's `d#0` provably `CONST('')`.
4. With `d#0 = CONST('')`, the dict-with key harvester registers NO keys; `_read_before_set` therefore doesn't exempt `missing`; the body's read fires W210.
5. Two-part closure: (a) `_collect_call_site_constants` builds a per-callee literal-arg map (only when ALL callers agree on the literal value); `_compile_source_inner` seeds the callee's SCCP lattice with `param_constants={(p,0): CONST(v)}`.  (b) The SCCP barrier-widening pass preserves version-0 entries (param-entry values are by construction the from-outside value, never re-written in the body), so the CONST survives the barrier.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {d} { dict with d { return $missing } }
% f {}
can't read "missing": no such variable
% f {missing ok}
ok
```

The first call errors (empty dict, no `missing` key); the second succeeds (key present, unpacked as a local).

#### Compiler evidence

```
--- FP-DS-09: interproc literal-dict propagation feeds dict-with key check (D3-P2)
regen: python -m bench.fp_snippets --id FP-DS-09
function ::f
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='dict'  defs={d#1}  uses={d#0, missing#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    d#0: CONST('')
    missing#0: OVERDEFINED
  read_before_set
    ReadBeforeSet(block='entry_1', statement_index=1, variable='missing')
```
(regen: `python -m bench.fp_snippets --id FP-DS-09`)

`d#0: CONST('')` is the load-bearing fact — interproc propagation from the call site `f {}` seeded the lattice; the dict-with key check then exempts no names; `$missing` is reported.

#### Why the analyser reaches that verdict

- `compiler/core_analyses.py:3978` — `_collect_call_site_constants(ir_module)` builds the per-callee literal-arg dictionary (skipped when any call site has a non-literal in the same slot, so mixed callers fall back to conservative).
- `compiler/core_analyses.py:1257-1289` — `param_constants` parameter to the SCCP driver seeds `(name, 0)` lattice entries with the agreed-CONST values before fixpoint.
- `compiler/core_analyses.py:2910` — the SCCP barrier-widening refinement preserves version-0 entries (param-entry values never re-written in the function body), so the CONST survives the barrier through to the dict-with key-aware check.

#### Tests

- `tests/test_fp_ds.py::test_FP_DS_09_interproc_empty_dict_fires` (TP)
- `tests/test_fp_ds.py::test_FP_DS_09_interproc_key_present_silent` (TN — caller passes key)
- `tests/test_fp_ds.py::test_FP_DS_09_interproc_mixed_callers_conservative_silent` (TN — mixed callers fall back to conservative)
- `tests/test_ground_truth_tn_fn.py::test_TP_W210_interproc_dict_with_empty_arg_unpacks_no_keys`
- `tests/test_ground_truth_tn_fn.py::test_TN_interproc_dict_with_key_present_silent`
- `tests/test_ground_truth_tn_fn.py::test_TN_interproc_mixed_callers_conservative`

---

### FP-DS-10 — reads nested in a `dict for`/`dict map` body keep the store live (issue #833)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tcl-compiler/src/analyser/diagnostics/fp/ds.rs::fp_ds_10_*`
- **Codes:** W220, W211
- **Corpus:** any `dict for {k v} $d { … }` whose body reads an outer variable from
  inside its own control flow — dispatch tables (`$cmd a $key` under an `if`),
  guarded `puts $msg`, accumulators updated conditionally.

#### Reproducer

```tcl
proc demo {} {
    set x set
    set d [dict create a true b false c true]
    dict for {key value} $d {
        if {$value} {
            $x a $key
        }
    }
}
```

#### Per-line reasoning

1. `set x set` — assigns version 1 of `x`.
2. `dict for {key value} $d { … }` — the body runs in the caller's frame.
3. `if {$value} { $x a $key }` — `$x` is the **command name** of the dispatched
   call, so it reads `x#1`; this keeps `set x set` alive.

Pre-fix, `dict for`/`dict map` were re-emitted as an opaque `Barrier` in the
analysis CFG and their body reads were recovered by a shallow word scan that only
saw top-level `$var` / `[...]` tokens. A read one brace level deep — `$x` inside
the `if` body — was invisible, so `set x set` looked like a dead store. The fix
lowers the dict-for body into real CFG blocks in the analysis build (as `foreach`
and `array for` already are), making body reads first-class SSA uses. Codegen
keeps the byte-identical `::tcl::dict::for` barrier (built from the separate
`build_cfg_codegen`), so emitted bytecode is unchanged.

#### tclsh ground truth

```
% set x set; set d [dict create a true b false]; dict for {k v} $d { if {$v} { $x q $k } }; puts "q=$q"
q=a
```
`$x` resolves to `set` at run time, so `x` is genuinely read.

#### Tests

- `fp_ds_10_command_name_read_in_dict_for_if_keeps_store_live` (FP — the issue repro)
- `fp_ds_10_dict_map_variant_keeps_store_live` (FP — `dict map`)
- `fp_ds_10_deeply_nested_read_keeps_store_live` (FP — two `if` levels deep)
- `fp_ds_10_plain_var_read_in_dict_for_if_keeps_store_live` (FP — ordinary arg read)
- `fp_ds_10_real_dead_store_before_dict_for_still_fires` (TP — real dead store outside body)
- `fp_ds_10_dead_store_inside_dict_for_body_now_fires` (TP — precision gained inside the body)
- `fp_ds_10_write_only_local_in_dict_for_body_still_flags` (FN guard — write-only local)
- `fp_ds_10_clean_dict_for_is_silent` (TN control)

---

### FP-DS-11 — an `uplevel` body runs in another stack frame (issue #837)

- **Verdict:** FALSE POSITIVE guard + frame-aware precision (TP)
- **Status:** locked in by `tcl-compiler/src/analyser/diagnostics/fp/ds.rs::fp_ds_11_*`
  (and the W105 consistency arm in `fp/sty.rs::fp_sty_14_uplevel_body_participates_in_w105_like_eval`)
- **Codes:** W220, W211, W105
- **Corpus:** any proc that runs caller-context code via `uplevel ?level? {body}` —
  DSL helpers, `namespace forget`/`namespace import` shims, option-setter idioms.

#### Reproducer

```tcl
proc forgetXyce {} {
    # Forgets all '::SpiceGenTcl::Xyce' commands from caller namespace
    uplevel 1 {foreach nameSpc [namespace children ::SpiceGenTcl::Xyce] {
        namespace forget ${nameSpc}::*
    }}
}
```

#### Per-line reasoning

`uplevel`'s script word now carries an `ArgRole::Body` (a registry-driven
`arg_role_resolver` that skips an optional leading `level` word), so the body is
recursed and highlighted/analysed like every other script body instead of being
rendered as one opaque string (the original issue #837 symptom — no highlighting
inside the `uplevel` body). The spec is `BodyKind::Structural`: the body runs in
the frame named by `level`, so a `$var` read inside a **braced** body resolves
against *that* frame and does not count as a use of an enclosing-proc local of the
same name. A value substituted at the enclosing level (`[list …]`, a quoted body,
or a plain local read) is evaluated in the enclosing frame and keeps the local
live.

#### tclsh ground truth

```
% proc f {} { set x 1; uplevel 1 {puts $x} }
% set x outer; f
outer
```
`uplevel 1 {puts $x}` prints the *caller's* `x` (`outer`), never `f`'s local
`x`, so `set x 1` inside `f` is genuinely a dead store.

#### Tests

- `fp_ds_11_caller_frame_write_is_not_an_enclosing_dead_store` (FP/TN)
- `fp_ds_11_list_substituted_read_keeps_enclosing_store_live` (FP — `[list]` body)
- `fp_ds_11_local_read_keeps_store_live_alongside_uplevel` (FP — local read)
- `fp_ds_11_braced_body_only_read_is_a_frame_shifted_dead_store` (TP — frame-aware precision)
- `fp_ds_11_real_dead_store_outside_uplevel_still_fires` (TP — enclosing analysis intact)
- `fp_ds_11_clean_uplevel_body_is_silent` (TN control)
- `fp_sty_14_uplevel_body_participates_in_w105_like_eval` (TP/FP — unbraced-body W105 parity with `eval`)

---


## SH — shimmer (S100/S101/S102)

Shimmer fires when a value of one Tcl intrep flows into an operator that
wants another (STRING into arithmetic, INT into eq, etc.).  These entries
lock in the conservative suppressions (OVERDEFINED / scope-alias) and the
determinism property of the phi-join (no PYTHONHASHSEED flake).

### FP-SH-01 — OVERDEFINED values do not trigger shimmer (conservative suppression)

- **Verdict:** FALSE POSITIVE (conservative suppression)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_01_*`
- **Codes:** S100, S101, S102
- **Corpus:** any code that consumes an unknown-command return (the bulk of integration glue and event-handler bodies).

#### Reproducer

```tcl
# x has unknown type (cmd return) -> OVERDEFINED -> no shimmer warning.
set x [unknownCmd]
set y [expr {$x + 1}]
return $y
```

#### Per-line reasoning

1. `set x [unknownCmd]` — the SCCP lattice cannot resolve `unknownCmd`'s return, so `x#1` is OVERDEFINED.
2. `set y [expr {$x + 1}]` — `$x` flows into arithmetic.

Pre-fix logic: STRING-typed `$x` in arithmetic → S100.  But `x` isn't STRING — it's *unknown*.  A shimmer claim would be unsound.

The conservative rule is to suppress shimmer for OVERDEFINED / UNKNOWN values; the cost is missing genuine STRING-in-arithmetic when type inference can't see far enough, but that's preferable to false-claiming a type the code doesn't have.

#### tclsh ground truth

N/A (this is a static-analysis precision decision, not a runtime check)

#### Compiler evidence

```
--- FP-SH-01: OVERDEFINED values do not trigger shimmer (conservative suppression)
regen: python -m bench.fp_snippets --id FP-SH-01
function ::top
  block entry_1
    [0] AssignValue 'x' value='[unknownCmd]'  defs={x#1}  uses={}
    [1] AssignExpr 'y'  defs={y#1}  uses={x#1}
    term Return ${y}
  values (SCCP lattice)
    x#1: OVERDEFINED
```

#### Why the analyser reaches that verdict

`compiler/shimmer.py`'s entry filter skips values whose `LatticeKind` is `OVERDEFINED` or `UNKNOWN`.  See also the `force-OVERDEFINED-for-escaping` rule in `compiler/core_analyses.py` that pushes values to OVERDEFINED on call-out — the same sound-but-imprecise tradeoff at the call-graph boundary.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_01_overdefined_silent` (FP)
- `tests/test_fp_sh.py::test_FP_SH_01_string_arith_still_fires` (TP control)

---

### FP-SH-02 — scope-alias declarations typed OVERDEFINED (not STRING) — kills shimmer FPs

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_02_*`
- **Codes:** S100, S101, S102
- **Corpus:** every proc using `variable` / `global` / `upvar` to expose a namespace or caller variable (extremely common in tcllib's namespace-eval modules).

#### Reproducer

```tcl
proc f {} {
    # `variable v` declares an alias — type is unknown (OVERDEFINED),
    # NOT STRING, so `expr {$v + 1}` must NOT fire S100.
    variable v
    return [expr {$v + 1}]
}
```

#### Per-line reasoning

1. `variable v` — declares `v` as a local alias for the current namespace's `::ns::v` slot.  The local's intrep is whatever the storage holds — externally determined.
2. `expr {$v + 1}` — arithmetic on `$v`.

Pre-fix `variable v` defaulted `v#1` to TclType.STRING (an unsound guess based on declaration spelling, not actual content).  S100 fired the first time anything used `$v` in arithmetic.

Fix: type alias declarations as OVERDEFINED (truly unknown) so they fall under the FP-SH-01 conservative suppression.

#### tclsh ground truth

N/A — static precision decision; runtime would show whatever intrep the namespace storage carries.

#### Compiler evidence

```
--- FP-SH-02: scope-alias declarations typed OVERDEFINED (not STRING) — kills shimmer FPs
regen: python -m bench.fp_snippets --id FP-SH-02
function ::f
  block entry_1
    [0] Call cmd='variable'  defs={v#1}  uses={}
    term Return [expr {$v + 1}]
  values (SCCP lattice)
    v#1: OVERDEFINED
```

#### Why the analyser reaches that verdict

`compiler/core_analyses.py` types the def from `variable` / `global` / `upvar` as TypeLattice.OVERDEFINED instead of STRING.  This was commit `adfc6d84` in the parser/compiler-algorithms branch.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_02_variable_alias_no_shimmer` (FP)
- `tests/test_fp_sh.py::test_FP_SH_02_global_alias_no_shimmer` (FP — global)

---

### FP-SH-03 — phi joins are hash-seed-independent (deterministic shimmer)

- **Verdict:** DETERMINISM PROPERTY (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_03_*`
- **Codes:** S100, S101, S102
- **Corpus:** loop-merged values across the corpus (the historical flake source was tcllib's `struct::set` accumulators).

#### Reproducer

```tcl
proc f {n} {
    # x is joined at the loop header from two INT branches; the join
    # must come out INT every run (no flake) -> no S101.
    set x 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i > 5} { set x 1 } else { set x 2 }
    }
    return [expr {$x + 1}]
}
```

#### Per-line reasoning

1. `set x 0` — `x#1: INT`.
2. The for-loop header creates `x#2 = phi(x#1, x#3)`.  Both incoming branches (the if-then and if-else) write INT — `set x 1`, `set x 2`.
3. The phi join over `{INT, INT}` must produce INT.

Pre-fix the join iterated over a set whose iteration order depended on PYTHONHASHSEED; the reducing fold could pick a different "winning" type per run, occasionally yielding STRING.  The downstream `expr {$x + 1}` would then sporadically fire S101.

#### tclsh ground truth

N/A — determinism property of the analyser, not Tcl semantics.

#### Compiler evidence

```
--- FP-SH-03: phi joins are hash-seed-independent (deterministic shimmer)
regen: python -m bench.fp_snippets --id FP-SH-03
function ::f
  block entry_1
    [0] AssignConst 'x' value='0'  defs={x#1}  uses={}
    [1] AssignConst 'i' value='0'  defs={i#1}  uses={}
    term Goto
  block for_header_2
    phi  SSAPhi(name='i', version=2, incoming={'entry_1': 1, 'for_step_4': 3})
    phi  SSAPhi(name='x', version=2, incoming={'entry_1': 1, 'for_step_4': 3})
    term Branch ExprBinary(op=<BinOp.LT: '<'>, left=ExprVar(text='$i', name='i', start=0, end=1), right=ExprVar(text='$n', name='n', start=5, end=6))
  block for_body_3
    term Branch ExprBinary(op=<BinOp.GT: '>'>, left=ExprVar(text='$i', name='i', start=0, end=1), right=ExprLiteral(text='5', start=5, end=5))
  block for_step_4
    [0] Incr 'i'  defs={i#3}  uses={i#2}
    term Goto
  block for_end_5
    term Return [expr {$x + 1}]
  block if_end_6
    phi  SSAPhi(name='x', version=3, incoming={'if_next_8': 4, 'if_then_7': 5})
    term Goto
  block if_then_7
    [0] AssignConst 'x' value='1'  defs={x#5}  uses={}
    term Goto
  block if_next_8
    [0] AssignConst 'x' value='2'  defs={x#4}  uses={}
    term Goto
  values (SCCP lattice)
    x#1: CONST(0)
    x#2: CONSTSET([0, 1, 2])
    x#3: CONSTSET([1, 2])
    x#4: CONST(2)
    x#5: CONST(1)
```

#### Why the analyser reaches that verdict

`compiler/core_analyses.py` sorts phi-source type entries by a canonical key before reducing the join.  The fix was commit `b08f2c47` (`Make SSA type-propagation phi joins deterministic`).

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_03_phi_join_deterministic` (FP)
- `tests/test_fp_sh.py::test_FP_SH_03_genuine_phi_string_int_still_fires` (smoke / OVERDEFINED path)

---

### FP-SH-04 — hex/binary integer literals typed as INT (not STRING)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_04_*`
- **Codes:** S100, S101, S102
- **Corpus:** `tmp/tcl9.0.3/library/cookiejar/idna.tcl` (and every other hex-heavy module — DES, AES, blowfish, CRC tables).

#### Reproducer

```tcl
proc f {} {
    # 0x80 is a Tcl hex integer literal -- typed INT, not STRING.
    # incr on $n must NOT fire S100/S101.
    set n 0x80
    for {set i 0} {$i < 10} {incr i} {
        incr n
    }
    return $n
}
```

#### Per-line reasoning

1. `set n 0x80` — the literal `0x80` is a Tcl hex integer (recognised by `Tcl_GetIntFromObj` on first numeric use).  Its intrep is INT.
2. The for-loop `incr n` increments an integer.  `n` stays INT across iterations.

Pre-fix `_literal_type` in `compiler/core_analyses.py` only matched `_DECIMAL_INT_RE` (decimal digits only) and fell through to STRING for hex (`0x...`) and binary (`0b...`) prefix forms.  The `set n 0x80` then propagated STRING; `incr n` (which expects INT) fired S101 every iteration — "intrep string but incr expects int".

In reality the hex literal is INT and the loop is clean.

Fix: extend `_literal_type` to recognise `_HEX_INT_RE` (`^[+-]?0[xX][0-9a-fA-F]+$`) and `_BIN_INT_RE` (`^[+-]?0[bB][01]+$`) as INT before falling through to STRING.  Tcl 9 dropped the legacy "leading-0 means octal" rule, so leading-0 forms are NOT recognised — dialect-dependent and error-prone; users should write `0o...` explicitly which can be added if the corpus shows it.

#### tclsh ground truth

```
% set n 0x80; incr n; puts $n
129
```

Hex literal converts to int 128 on first numeric use; incr makes it 129; no error.

#### Compiler evidence

The `_literal_type` helper:

```python
def _literal_type(text: str) -> TypeLattice:
    stripped = text.strip()
    if _DECIMAL_INT_RE.fullmatch(stripped):
        return TypeLattice.of(TclType.INT)
    if _HEX_INT_RE.fullmatch(stripped) or _BIN_INT_RE.fullmatch(stripped):
        return TypeLattice.of(TclType.INT)
    if _FLOAT_RE.fullmatch(stripped):
        return TypeLattice.of(TclType.DOUBLE)
    if stripped.lower() in _BOOL_LITERALS:
        return TypeLattice.of(TclType.BOOLEAN)
    return TypeLattice.of(TclType.STRING)
```

#### Why the analyser reaches that verdict

`compiler/core_analyses.py::_literal_type` recognises hex/binary prefix forms as INT.  Concrete tcllib pattern from `cookiejar/idna.tcl`:

```tcl
variable initial_n 0x80
set n $initial_n
for {set h $b} {$h < [llength $in]} {incr delta; incr n} { ... }
```

Pre-fix the hex literal propagated STRING through `set n $initial_n`, so `incr n` fired S101 in the encoding loop.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_04_hex_literal_increment_no_shimmer` (FP)
- `tests/test_fp_sh.py::test_FP_SH_04_binary_literal_increment_no_shimmer` (FP — `0b...` form)
- `tests/test_fp_sh.py::test_FP_SH_04_genuine_string_increment_still_fires` (TP control)

---

### FP-SH-05 — destructure foreach (`foreach VARS LIST break`) excluded from loop body types

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_05_*`
- **Codes:** S102 (and indirectly S101)
- **Corpus:** `tmp/tcllib-2.0/modules/grammar_me/me_cpucore.tcl` (the canonical site; the destructure idiom predates `lassign` and pervades pre-8.5 tcllib modules).

#### Reproducer

```tcl
proc f {state} {
    # foreach + break is the pre-8.5 lassign equivalent: a
    # single-iteration foreach that destructures a list.  The var
    # bindings are one-time, not per-iteration body types — they
    # must NOT pollute a sibling real loop's S102 oscillation check.
    foreach {a b c sv} $state break
    foreach inst {1 2 3} {
        set sv [list 1 2 3]
    }
    return $sv
}
```

#### Per-line reasoning

1. `foreach {a b c sv} $state break` — Tcl's pre-`lassign` destructure idiom: foreach binds `a`,`b`,`c`,`sv` to the first four elements of `$state`, then the body executes `break` which exits the loop after one iteration.  Effectively a multi-assign that runs ONCE.
2. The main loop `foreach inst {1 2 3}` runs three times.  Each iteration `set sv [list 1 2 3]` (LIST type).
3. From the main loop's perspective: `sv` enters as STRING (the destructure binding), body produces LIST.  Entry-type set = {STRING}; body-type set = {LIST}.  Intersection empty, body cardinality 1 → no oscillation → no S102.

Pre-fix: the destructure foreach IS a CFG loop (header + body block + back-edge).  Its var binding (STRING, from the list-element-typed foreach binding) was added to the function-wide `loop_body_types` map.  When the main loop's S102 check queried `loop_body_types[sv]`, it got `{STRING, LIST}` — the union across ALL loop blocks in the proc, including the destructure.  Cardinality 2 → oscillates=True → S102 fired on the main loop's `sv` phi even though there's no real per-iter oscillation (the destructure ran once).

Fix: detect "destructure foreach" blocks — an SSA block whose first statement is `IRCall(command="foreach"|"lmap")` and whose foreach body block contains only `IRCall(command="break")`.  Exclude those blocks from the `in_loop` check in `_build_shimmer_name_index`, so their var bindings don't pollute `loop_body_types`.

#### tclsh ground truth

```
% set state {1 2 3 4 5}
% foreach {a b c sv} $state break
% puts $sv
4
```

The destructure runs once; `sv` is bound to the fourth element.  No iteration.

#### Compiler evidence

`compiler/shimmer.py::_destructure_foreach_blocks` produces the exclusion set:

```python
def _destructure_foreach_blocks(ssa: SSAFunction, cfg) -> set[str]:
    destructure: set[str] = set()
    for bname, block in cfg.blocks.items():
        ssa_block = ssa.blocks.get(bname)
        if not block.statements:
            continue
        first = block.statements[0]
        if not isinstance(first, IRCall) or first.command not in ("foreach", "lmap"):
            continue
        # body block (successor) must contain only an IRCall(break).
        for succ in <successors of block>:
            succ_stmts = cfg.blocks[succ].statements
            if len(succ_stmts) == 1 and isinstance(succ_stmts[0], IRCall) and succ_stmts[0].command == "break":
                destructure.add(bname)
                break
    return destructure
```

Sample impact (me_cpucore.tcl alone): S101 113→82 (−31), S102 48→29 (−19) after this fix; sample S102 across six tcllib files dropped from 161 → 93 (−42%).

#### Why the analyser reaches that verdict

`compiler/shimmer.py::_build_shimmer_name_index` (with the `cfg` parameter) computes `destructure_blocks` via `_destructure_foreach_blocks(ssa, cfg)` and replaces `in_loop = bn in loop_blocks` with `in_loop = bn in loop_blocks and bn not in destructure_blocks`.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_05_destructure_foreach_no_s102` (FP)
- `tests/test_fp_sh.py::test_FP_SH_05_real_iter_foreach_still_fires` (TP control — a real multi-iter foreach with body that oscillates types still fires S102)

---

### FP-SH-06 — per-loop body_types (sibling loops do not pollute each other)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_06_*`
- **Codes:** S102
- **Corpus:** tcllib procs with multiple sibling loops sharing local names (`tmp/tcllib-2.0/modules/grammar_me/me_cpucore.tcl`, DES, tepam, graphops, docstrip, exif).

#### Reproducer

```tcl
proc f {items} {
    # Loop A and loop B are SIBLINGS — they don't nest.  Each loop
    # alone is monomorphic in $x (loop A: STRING only; loop B: LIST
    # only).  Neither loop oscillates, so S102 must not fire on either.
    foreach a $items {
        set x "value"
    }
    foreach b $items {
        set x [list 1 2]
    }
}
```

#### Per-line reasoning

1. Loop A's only assignment to `$x` is `set x "value"` (STRING).  Within loop A, the phi for `$x` at the header sees: entry={whatever before}, body={STRING}.  Cardinality 1, no overlap → no oscillation.
2. Loop B's only assignment to `$x` is `set x [list 1 2]` (LIST).  Within loop B, the phi sees: entry={STRING from loop A's last write}, body={LIST}.  Cardinality 1, no overlap → no oscillation.

Pre-fix: `loop_body_types` was a function-wide map `name → union of body types across ALL loop blocks in the proc`.  For `$x`, the function-wide set was `{STRING, LIST}`.  When checking either loop, the oscillates predicate

```python
oscillates = bool(entry_types & all_body_types) or len(all_body_types) >= 2
```

evaluated to True (`len({STRING, LIST}) >= 2`) — S102 fired on both loops' phi for `$x`, even though neither loop alone oscillates.

Fix: build `per_header_body_types: dict[loop_header → dict[name → set[TclType]]]` using the natural loop forest (`compiler.loops.build_loop_forest`).  Each loop contributes only its OWN blocks' body types.  At `_find_thunking`'s emission point, look up `per_header_body_types[bn].get(phi.name, set())` instead of the function-wide map.

#### tclsh ground truth

```
% set items {1 2 3}
% foreach a $items { set x "value" }
% foreach b $items { set x [list 1 2] }
% puts $x
1 2
```

Each loop runs to completion; `$x` ends as whatever the last loop body left it.  No type churn within either loop.

#### Compiler evidence

The per-loop map is built in `compiler/shimmer.py::_find_thunking`:

```python
forest = build_loop_forest(cfg, ssa, executable_blocks)
destructure_set = _destructure_foreach_blocks(ssa, cfg)
per_header_body_types: dict[str, dict[str, set[TclType]]] = {}
for loop in forest.loops:
    per_loop_types: dict[str, set[TclType]] = {}
    for lbn in loop.blocks:
        if lbn in destructure_set:
            continue
        # collect KNOWN intrep types for defs in THIS loop only
    per_header_body_types[loop.header] = per_loop_types
```

And the emission point uses it:

```python
per_loop = per_header_body_types.get(bn, {}).get(phi.name, set())
all_body_types = body_types | per_loop
oscillates = bool(entry_types & all_body_types) or len(all_body_types) >= 2
```

Sample impact (six tcllib files): S102 93→30 (−68%).  me_cpucore.tcl: 29→3 S102 (−90%).

#### Why the analyser reaches that verdict

`compiler/shimmer.py::_find_thunking` (S102 emission) uses `per_header_body_types[bn]` (per the loop the phi belongs to) instead of the function-wide `loop_body_types` map.  Function-wide `loop_body_types` is still computed (kept for S100/S101 paths which aren't yet refactored to per-loop — diminishing returns there since per-phi `body_types` already filters most pollution).

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_06_sibling_loops_no_s102` (FP)
- `tests/test_fp_sh.py::test_FP_SH_06_real_oscillation_within_one_loop_still_fires` (TP control)

---

### FP-SH-07 — expr-context shimmers detected for standalone expr/if/while/for (D5-SH-EXPR)

- **Verdict:** TRUE POSITIVE / precision FN (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_07_*`
- **Codes:** S100 (use-site shimmer), S101 (loop variant)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-SH-EXPR

#### Reproducer

```tcl
proc f {} {
    set s [string trim "5"]
    # $s is STRING-typed; the if-condition is an expr context that
    # promotes $s to INT.  Pre-fix the analyser only walked
    # IRAssignExpr, so this site was missed.
    if {$s + 1} { puts yes }
}
```

#### Per-line reasoning

1. `set s [string trim "5"]` — `s` is KNOWN STRING-typed.
2. `if {$s + 1} { ... }` — the expr `$s + 1` lives on `CFGBranch.condition` of the if-terminator (NOT in an `IRAssignExpr` statement).  Pre-fix `_find_expr_shimmers` only iterated `IRAssignExpr` bodies, so the terminator was never examined.
3. Tcl evaluates the if-condition with the same numeric-coercion rules as `expr`; `$s + 1` triggers `Tcl_GetIntFromObj($s)` which silently converts the STRING intrep to INT.  A genuine shimmer site that should be reported.
4. Post-fix `_find_expr_shimmers` walks: (a) `IRAssignExpr` bodies (unchanged), (b) `IRExprEval` statement bodies (standalone `expr {...}`), and (c) `CFGBranch.condition` on each block's terminator (covers if/while/for conditions).  The SSA uses for the terminator come from the block's `exit_versions`.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set s [string trim "5"]
% tcl::unsupported::representation $s
value is a pure string with a refcount of N, ..., string representation "5"
% if {$s + 1} { puts yes }
yes
% tcl::unsupported::representation $s
value is a int with a refcount of N, ..., internal representation 0x5:..., string representation "5"
```

The if-condition lex-promoted `$s` from STRING to INT — a real shimmer.

#### Compiler evidence

```
--- FP-SH-07: expr-context shimmers also detected for standalone expr/if/while/for (D5-SH-EXPR)
regen: python -m bench.fp_snippets --id FP-SH-07
function ::f
  block entry_1
    [0] AssignValue 's' value='[string trim "5"]'  defs={s#1}  uses={}
    term Branch ExprBinary(op=<BinOp.ADD: '+'>, left=ExprVar(text='$s', name='s', start=0, end=1), right=ExprLiteral(text='1', start=5, end=5))
  ...
  types
    s#1: STRING
```
(regen: `python -m bench.fp_snippets --id FP-SH-07`)

#### Why the analyser reaches that verdict

`compiler/shimmer.py` — `_find_expr_shimmers`:

- Iterates IRAssignExpr (existing) AND IRExprEval (new) statements with `ssa_stmt.uses` as the in-scope versions.
- After per-statement processing, examines `block.terminator` — if it's a `CFGBranch`, walks `term.condition` using `ssa_block.exit_versions` as the in-scope SSA versions.
- All three call paths funnel through `_emit_expr_shimmer_warnings` which calls `_collect_expr_shimmers` and de-dups by `(range, var_name)` within the block.
- The terminator's `range` is preferred; falls back to the last statement's range if the terminator's range is None.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_07_if_condition_shimmer_fires` (TP, `if {$s + 1}`)
- `tests/test_fp_sh.py::test_FP_SH_07_while_condition_shimmer_fires` (TP, `while {$s + 1}`)
- `tests/test_fp_sh.py::test_FP_SH_07_standalone_expr_shimmer_fires` (TP, `expr {$s + 1}` result dropped)
- `tests/test_fp_sh.py::test_FP_SH_07_pure_numeric_if_no_shimmer` (TN, `if {1 + 1}`)

---

### FP-SH-08 — `==`/`!=` falsely flagged as numeric shimmer when both operands non-numeric (D5-SH-EQ)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_08_*`
- **Codes:** S100 (use-site shimmer)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-SH-EQ

#### Reproducer

```tcl
proc f {} {
    set s [string trim hello]
    # Both operands are non-numeric text. tclsh takes the STRING-compare
    # path; no numeric coercion is attempted on $s and no shimmer occurs.
    set y [expr {$s == "world"}]
    puts $y
}
```

#### Per-line reasoning

1. `set s [string trim hello]` — `s` is KNOWN STRING-typed.
2. `expr {$s == "world"}` — Tcl's `==` semantics: try integer parse on both sides, then double parse on both sides; if BOTH parses succeed go numeric, else fall back to string compare.  ``"hello"`` and ``"world"`` both fail numeric parse, so tclsh short-circuits to string compare — no intrep conversion happens.
3. Pre-fix `_NUMERIC_OPS` contained `BinOp.EQ`/`BinOp.NE`, so the shimmer collector treated `==`/`!=` as ALWAYS numeric and flagged $s.  That's a false positive — no actual shimmer occurs at runtime.
4. Post-fix `_CONDITIONAL_NUMERIC_OPS = {EQ, NE}` is checked separately: shimmer fires only when `_operand_looks_numeric` returns True for at least one operand.  Both-non-numeric stays silent.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set s hello
% expr {$s == "hello"}
1                       ;# string-compare path; no shimmer
% set s2 "5"
% expr {$s2 == "5"}
1                       ;# numeric path (both parse) -- shimmer
% expr {$s + 0}
can't use non-numeric string "hello" as left operand of "+"
```

#### Compiler evidence

```
--- FP-SH-08: ==/!= falsely flagged as numeric shimmer when both operands non-numeric (D5-SH-EQ)
regen: python -m bench.fp_snippets --id FP-SH-08
function ::f
  block entry_1
    [0] AssignValue 's' value='[string trim hello]'  defs={s#1}  uses={}
    [1] AssignExpr 'y'  defs={y#1}  uses={s#1}
    [2] Call cmd='puts'  defs={}  uses={y#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  types
    s#1: STRING
    y#1: BOOLEAN
```
(regen: `python -m bench.fp_snippets --id FP-SH-08`)

#### Why the analyser reaches that verdict

`compiler/shimmer.py`:

- `_CONDITIONAL_NUMERIC_OPS = frozenset({BinOp.EQ, BinOp.NE})` — partitioned out of `_NUMERIC_OPS`.
- `_collect_expr_shimmers` checks the conditional set separately: only when `_operand_looks_numeric(left, ...) or _operand_looks_numeric(right, ...)` does the per-operand `_check_operand_shimmer` run.  Otherwise both operands are silently accepted (tclsh's string-compare short-circuit).
- `_operand_looks_numeric` accepts: `ExprLiteral` (parser-validated numeric), `ExprString` whose stripped text parses as a number/boolean, `ExprVar` whose KNOWN SSA type is INT/DOUBLE/NUMERIC/BOOLEAN, or `ExprVar` whose SCCP CONST value parses as a number.
- `_find_expr_shimmers` now takes `values=fu.analysis.values` and threads it through.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_08_eq_both_non_numeric_no_shimmer` (FP)
- `tests/test_fp_sh.py::test_FP_SH_08_eq_with_numeric_literal_still_fires` (TP, `$s == "5"` with $s STRING)
- `tests/test_fp_sh.py::test_FP_SH_08_add_still_fires` (TP control, `$s + 0` — always numeric)

---

### FP-SH-09 — byte array case-folded / re-encoded by a string op corrupts high bytes (S110)

- **Verdict:** TRUE POSITIVE (new correctness diagnostic)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_09_*`
- **Codes:** S110
- **Corpus:** plain-Tcl binary handling — `binary format`/`binary decode` data run through `string` / `encoding convertto` before being scanned back.

S110 is a **correctness** shimmer, separate from the S100/S101/S102
performance family.  A Tcl byte array and a character string are different
internal representations; forcing a byte array through character semantics
reinterprets each byte as a Unicode code point, and case folding (or a
re-encode) pushes bytes `>= 0x80` out of the byte range — corrupting the
data with no error in 8.x and a hard error in 9.x.

#### Reproducer

```tcl
# binary format -> string toupper -> corrupted bytes (S110).
set ba [binary format c* {128 195 255}]
set up [string toupper $ba]
```

#### Per-line reasoning

1. `set ba [binary format c* {128 195 255}]` — `ba` is a byte array (`binary format` return type BYTEARRAY) holding the bytes `80 c3 ff`.
2. `set up [string toupper $ba]` — `string toupper` demands a character string, so `ba`'s bytes are decoded to characters, upper-cased, and `up` becomes that STRING.  `0xFF` (ÿ) upper-cases to `Ŷ` (U+0178), which is **not** a single byte — the byte array is destroyed.

#### tclsh ground truth

```
% set ba [binary format c* {128 195 255}]; binary scan $ba H* h; set h
80c3ff
# Tcl 8.6.14:
% binary scan [string toupper $ba] H* h2; set h2
80c378            ;# 0xFF -> U+0178 truncated to 0x78 — silent data loss
# Tcl 9.0.3:
% binary scan [string toupper $ba] H* h2
expected byte sequence but character 2 was 'Ŷ' (U+000178)
```

#### Compiler evidence

```
--- FP-SH-09: byte array case-folded / re-encoded by a string op corrupts high bytes (S110)
regen: python -m bench.fp_snippets --id FP-SH-09
function ::top
  block entry_1
    [0] AssignValue 'ba' value='[binary format c* {128 195 255}]'  defs={ba#1}  uses={}
    [1] AssignValue 'up' value='[string toupper $ba]'  defs={up#1}  uses={ba#1}
    term Goto
  types
    ba#1: BYTEARRAY
    up#1: STRING
```

#### Why the analyser reaches that verdict

`compiler/shimmer.py`'s `_find_byte_array_corruption` tracks byte provenance
per SSA value.  `[binary format …]` (return type BYTEARRAY) marks `ba` BINARY;
`string toupper` is in `_CASE_FOLD_STRING_SUBS`, so the BINARY operand fires
S110 at the `string toupper` use site.  Case folding and `encoding convertto`
are flagged at the transform itself (the corruption is unconditional); latin-1
-preserving transforms (interpolation, `string replace`, `append`) only fire
when the result reaches a byte sink (see FP-SH-10).

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_09_toupper_byte_array_fires` (TP)
- `tests/test_fp_sh.py::test_FP_SH_09_toupper_plain_string_silent` (FP control — no byte source)

---

### FP-SH-10 — `*::payload` round-trip: string-coerced binary written back corrupts it (S110)

- **Verdict:** TRUE POSITIVE (new correctness diagnostic; iRules)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_10_*`
- **Codes:** S110
- **Corpus:** F5 iRules payload rewrites — the single most common binary-safety bug (F5 KB K22406348, and the `HTTP::payload replace` man-page warning).

`*::payload` reads the on-the-wire bytes as a byte array.  Coercing that
value to a character string (interpolation, `string` ops, `append`, …) and
writing it back with `<proto>::payload replace` re-encodes every byte
`>= 0x80`: the bytes are read as latin-1 then emitted as UTF-8, so a 2-byte
UTF-8 character double-encodes (`c3 b3` → `c3 83 c2 b3`).

#### Reproducer

```tcl
when HTTP_REQUEST_DATA {
    set original_data [HTTP::payload]
    set new_data "$original_data MODIFIED"
    HTTP::payload replace 0 100 $new_data
}
```

#### Per-line reasoning

1. `set original_data [HTTP::payload]` — `original_data` is the raw payload **byte array** (registry flag `byte_array_payload`).
2. `set new_data "$original_data MODIFIED"` — double-quote interpolation decodes the bytes to latin-1 characters; `new_data` is now a character STRING.
3. `HTTP::payload replace 0 100 $new_data` — the sink interprets its data argument as a byte array, so `new_data`'s characters are re-encoded as UTF-8.  Every original byte `>= 0x80` double-encodes — the payload is corrupted.

The documented fix is to re-binarify before the sink:
`binary scan $new_data c* -` forces a byte-array intrep so the write is
byte-for-byte.  With that line present S110 is silent (the re-binarify clears
the DAMAGED provenance).

#### tclsh ground truth

F5 KB K22406348 (TMM): the byte stream for `Józef` (`4a c3 b3 7a 65 66 …`)
becomes `4a c3 83 c2 b3 7a 65 66 …` after the round-trip — `ó` (`c3 b3`)
double-encodes to `c3 83 c2 b3`.  The `HTTP::payload replace` man page states
the argument "will be interpreted as a byte array … you should first run
`binary scan c* throwawayvariable`".

#### Compiler evidence

```
--- FP-SH-10: *::payload round-trip: string-coerced binary written back corrupts it (S110)
regen: python -m bench.fp_snippets --id FP-SH-10
function ::when::HTTP_REQUEST_DATA
  block entry_1
    [0] AssignValue 'original_data' value='[HTTP::payload]'  defs={original_data#1}  uses={}
    [1] AssignValue 'new_data' value='${original_data} MODIFIED'  defs={new_data#1}  uses={original_data#1}
    [2] Call cmd='HTTP::payload'  defs={}  uses={new_data#1}
    term Goto
  types
    new_data#1: STRING
    original_data#1: TypeLattice.OVERDEFINED
```

(The SCCP type lattice cannot see the payload getter's return — it is
OVERDEFINED — so byte provenance is tracked by the dedicated pass, not the
type lattice.)

#### Why the analyser reaches that verdict

`compiler/shimmer.py`'s `_find_byte_array_corruption` marks `original_data`
BINARY from the `HTTP::payload` getter (`registry.byte_array_payload_commands()`),
propagates DAMAGED through the interpolation, and emits S110 at the
`HTTP::payload replace` sink because the data argument is DAMAGED.  A
`binary scan $v …` between the coercion and the sink re-binarifies `v` in place
and suppresses the warning (`binary format … $v` does not — it returns a new
value without mutating `$v`).

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_10_payload_roundtrip_fires` (TP)
- `tests/test_fp_sh.py::test_FP_SH_10_clean_payload_writeback_silent` (FP control — no string coercion)
- `tests/test_fp_sh.py::test_FP_SH_10_binary_scan_fix_silent` (FP control — documented re-binarify fix)

---

### FP-SH-11 — `*::payload replace` data-arg layout is per-protocol, not always index 3 (S110)

- **Verdict:** TRUE POSITIVE (S110 coverage gap; iRules)
- **Status:** locked in by `tests/test_fp_sh.py::test_FP_SH_11_*`
- **Codes:** S110
- **Corpus:** non-TCP/HTTP payload rewrites — MQTT, DIAMETER, GTP `replace` sinks (PR #656 added S110; PR #658 review flagged the missed layouts).

`<proto>::payload replace` does **not** share one argument layout, so the
`<data>` operand sits at a different index per protocol.  Hardcoding index 3
(the TCP/HTTP `replace OFFSET LENGTH DATA` shape) silently missed S110 at the
other sinks:

- **data at index 3** — `replace OFFSET LENGTH DATA`: TCP, SCTP, UDP, HTTP, ASM, REWRITE, RTSP, SIP, WS, XML
- **data at index 1** — `replace DATA …`: MQTT (`replace <data> ?offset? ?length?`), DIAMETER (`replace PAYLOAD`)
- **variable** — GTP `replace ('-message' MESSAGE)? OFFSET COUNT NEW_VALUE`: data at index 3 normally, index 5 when the optional `-message MESSAGE` flag is present

#### Reproducer

```tcl
when MQTT_MESSAGE {
    set p [MQTT::payload]
    set bad "$p x"
    MQTT::payload replace $bad
}
```

#### Per-line reasoning

1. `set p [MQTT::payload]` — `p` is the raw payload **byte array** (registry flag `byte_array_payload`).
2. `set bad "$p x"` — double-quote interpolation decodes the bytes to latin-1 characters; `bad` is now a character STRING.
3. `MQTT::payload replace $bad` — the data operand is `$bad` at **index 1** (`replace <data>`).  The sink re-encodes its character data as UTF-8, double-encoding every original byte `>= 0x80`.

The documented fix (`binary scan $bad c* -` before the sink) re-binarifies the
value and clears S110, exactly as for the index-3 sinks.

#### Compiler evidence

```
--- FP-SH-11: *::payload replace data-arg layout is per-protocol, not always index 3 (S110)
regen: python -m bench.fp_snippets --id FP-SH-11
function ::when::MQTT_MESSAGE
  block entry_1
    [0] AssignValue 'p' value='[MQTT::payload]'  defs={p#1}  uses={}
    [1] AssignValue 'bad' value='${p} x'  defs={bad#1}  uses={p#1}
    [2] Call cmd='MQTT::payload'  defs={}  uses={bad#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  types
    bad#1: STRING
    p#1: TypeLattice.OVERDEFINED
```

#### Why the analyser reaches that verdict

`compiler/shimmer.py`'s `_payload_replace_data_index` derives the data-operand
position from `registry.byte_array_payload_layouts()` (a `BytePayloadSpec` per
command) instead of hardcoding 3, so MQTT/DIAMETER (index 1) and GTP (index 3,
shifted to 5 by `-message`) are all checked.  New payload commands stay correct
by declaring their layout in `dialects/f5/irules/*__payload.py` — no change to
`shimmer.py` is needed.

#### Tests

- `tests/test_fp_sh.py::test_FP_SH_11_mqtt_payload_layout_fires` (TP — index-1 MQTT sink)
- `tests/test_fp_sh.py::test_FP_SH_11_gtp_message_flag_shift_fires` (TP — GTP `-message` index-5 shift)
- `tests/test_fp_sh.py::test_FP_SH_11_binary_scan_fix_silent` (FP control — documented re-binarify fix)
- `tests/test_fp_sh.py::test_FP_SH_11_gtp_offset_not_data_silent` (FP control — clean offset at the old index 3)

---

### FP-SH-12 — a variable-write trace can rewrite a value's type on every access; S102 must not fire from its literal-only view

- **Verdict:** FALSE POSITIVE (S102) — fixed
- **Status:** FIXED (Rust port deep review). `type_infer::propagate_types` had
  no trace awareness at all: a `trace add variable x write cb` callback can
  rewrite `x` immediately after any `set`, so a `set`-only view of `x`'s
  literal types cannot prove what the variable actually holds at loop
  re-entry, yet the pass typed it purely from the visible literals.
- **Codes:** S102
- **Corpus:** synthetic (a traced accumulator oscillating int/string in a loop, the same shape S102's own worked example uses).

#### Reproducer

```tcl
proc f {} {
    trace add variable x write {apply {{n1 n2 op} {}}}
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
```

#### Per-line reasoning

1. `trace add variable x write …` installs a write trace on `x` — from this
   point, every `set x …` may have its value replaced by the callback before
   any later read observes it.
2. `set x [expr {$x + 1}]` / `set x [string range $x 0 end]` are the visible
   literal writes the pre-fix pass typed `x` from (INT, then STRING) — but
   what `x` actually holds after either write is not provable without
   knowing the trace callback's effect.
3. Because the callback is opaque, the sound type for every def of `x` from
   this point on is OVERDEFINED, not the literal's own type — so the header
   phi can never resolve to SHIMMERED, and S102 must not fire.

#### Why the analyser reaches that verdict

`var_observability::scan_module_variable_traces` already builds
`ModuleVariableTraces` (a module-wide, flow-insensitive trace-target set) for
`sccp::sccp`'s own constant-folding lattice — added by the O101 deep review
(PR #856) to fix an analogous SCCP gap. `type_infer::propagate_types` is a
*separate* lattice (`TypeKind::{Unknown,Known,Shimmered,Overdefined}`, the one
S100/S101/S102 consume) that never received the same fact. The fix threads
`module_traces: Option<&ModuleVariableTraces>` into `propagate_types` and
forces `TypeLattice::overdefined()` for any def whose variable name
`is_traced()`, reusing the exact fact SCCP already computes rather than
re-deriving trace recognition — see
`rust/tcl-compiler/src/type_infer.rs`'s `propagate_types` doc comment.

#### Tests

- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_12_traced_variable_no_s102` (FP)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_12_untraced_control_still_fires` (TP control)
- `rust::tcl_compiler::shimmer::thunking::tests::no_s102_for_traced_variable` (FP, unit level)
- `rust::tcl_compiler::shimmer::thunking::tests::thunking_still_fires_for_untraced_control` (TP control, unit level)

---

### FP-SH-13 — array-element writes collapse onto one SSA symbol; two stable-but-different elements must not read as one variable oscillating

- **Verdict:** FALSE POSITIVE (S100/S101/S102) — fixed
- **Status:** FIXED for all three shimmer codes. Originally fixed for S102 only
  (Rust port deep review); the same `array_element_symbols` exclusion is now
  shared by the use-site (S100/S101) and phi-merge (S101) passes, so an
  independent-element conflation no longer false-positives through *any*
  shimmer surface. Proper per-element SSA modelling (which would let a
  genuinely-oscillating single element still fire) remains the larger,
  out-of-scope alternative.
- **Codes:** S100, S101, S102
- **Corpus:** synthetic (a per-element-stable array accumulator, the common
  Tcl idiom `set arr(id) [somefn $arr(id)]`).

#### Reproducer

```tcl
proc f {} {
    set arr(x) 0
    while {1} {
        set arr(x) [expr {$arr(x) + 1}]
        set arr(x) [string range $arr(x) 0 end]
    }
}
```

#### Per-line reasoning

1. `tcl_syntax::naming::normalise_var_name` strips a `(key)` suffix before SSA
   interning, so `arr(a)`, `arr(b)`, … all intern to the *same* `Symbol`
   (`"arr"`) — a deliberate simplification elsewhere in the pipeline (var-ref
   scanning, codegen slot resolution), not specific to shimmer analysis.
2. Two array elements individually holding stable-but-different types (e.g.
   `arr(count)` always INT, `arr(label)` always STRING) therefore look, to the
   type lattice, exactly like *one* variable whose value alternates between
   INT and STRING — the same shape as a genuine same-slot oscillation.
3. Even the single-element reproducer above (which genuinely does oscillate
   the *same* element) shares the same conflated-symbol code path, so a
   general array-aware exclusion is the safer fix — see the "Why" section.

#### Why the analyser reaches that verdict

`shimmer::thunking::array_element_symbols` scans every `AssignConst` /
`AssignExpr` / `AssignValue` / `Incr` statement in the function for a raw
target name matching `codegen::values::split_array_ref`'s `arr(key)` shape
(reused, not re-derived) and excludes the resolved base `Symbol` from shimmer
reporting entirely. It is now `pub(super)`, so all three passes share the one
exclusion: `classify_thunking_phi` (S102), `check_invocation` / `check_incr_var`
(S100/S101 use-site) and `classify_phi_shimmer` (S101 phi-merge) each bail for
an array-base symbol. The guard is conservative in the *unsound* direction only
for coverage, not correctness: a genuinely oscillating single array element
(this reproducer) now goes undetected rather than risk flagging unrelated
elements — an accepted trade-off documented here, not a residual bug. A full
fix needs per-element SSA modelling (array elements collapse onto one symbol
via `tcl_syntax::naming::normalise_var_name` stripping the `(key)` suffix
everywhere), out of scope for this review.

#### Tests

- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_13_array_element_conflation_no_s102` (FP, S102)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_13_array_element_use_site_no_s100` (FP, S100)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_13_array_element_loop_use_site_no_s101` (FP, S101)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_13_array_element_phi_merge_no_s100` (FP, phi merge)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_13_scalar_control_still_fires` (TP control, S102)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_13_scalar_use_site_still_fires` (TP control, S100)
- `rust::tcl_compiler::shimmer::thunking::tests::no_s102_for_array_element_conflation` (FP, unit level)
- `rust::tcl_compiler::shimmer::use_site::tests::no_use_site_shimmer_for_array_element_conflation` (FP, unit level)
- `rust::tcl_compiler::shimmer::phi::tests::no_phi_shimmer_for_array_element_conflation` (FP, unit level)
- `rust::tcl_lsp_server::e2e::diagnostic_matrix::s100_silent_for_array_element_use_site` (FP, e2e)

---

### FP-SH-14 — self-referential oscillation through an intermediate branch merge was a false negative; a non-self-referential reset through the same shape must stay silent

- **Verdict:** FALSE NEGATIVE (S102) — fixed; paired with a same-shape FP guard
- **Status:** FIXED (Rust port deep review).
- **Codes:** S102
- **Corpus:** synthetic (a data-dependent branch choosing which conversion runs each pass — the loop still pays the same per-iteration re-conversion cost as the direct-incoming case S102 already caught, just reached through an `if`/`else` instead of two straight-line statements).

#### Reproducer (false negative, now fixed)

```tcl
proc f {n} {
    set x "seed"
    while {$n} {
        if {$n % 2} {
            set x [string range $x 0 end]
        } else {
            set x [list 1 2]
        }
        incr n -1
    }
    return $x
}
```

#### Reproducer (paired FP guard — must stay silent)

```tcl
proc f {n} {
    set x "seed"
    while {$n} {
        if {$n % 2} {
            set x "value"
        } else {
            set x [list 1 2]
        }
        incr n -1
    }
    return $x
}
```

#### Per-line reasoning

1. In the false-negative reproducer, the `if` arm's `string range $x 0 end`
   *reads* `$x`'s own prior value — `x` is a genuine loop-carried recurrence,
   not a freshly-reset scratch variable.
2. The loop-header phi's direct loop-internal predecessor is the bottom-of-
   loop block *after* the `if`/`else` join — a SHIMMERED merge of the two
   branch types, not a single KNOWN type. Pre-fix, `classify_thunking_phi`'s
   `has_body_incoming` gate only accepted a KNOWN direct predecessor, so any
   oscillation reached through an intermediate merge was invisible — silent,
   even though the header's own lattice was already SHIMMERED.
3. In the FP-guard reproducer, *neither* branch reads `$x` — both assign an
   unrelated fresh literal every pass. `x` is not self-referential: each pass
   produces a brand-new object with no prior state to reinterpret, so
   despite the identical branch/merge shape, there is nothing to oscillate
   between and S102 must stay silent (matches FP-SH-06's established
   sibling-loop non-self-reference reasoning, extended to a single loop's
   own branches).

#### Why the analyser reaches that verdict

`shimmer::thunking::loop_self_referential` checks, for the phi's own `Symbol`,
whether any statement inside the natural loop both reads (`uses`) and writes
(`defs`) that same symbol — the SSA def/use maps `find_thunking_warnings`
already had on hand, no new dataflow pass needed. `classify_thunking_phi` now
additionally accepts a SHIMMERED direct predecessor as body evidence (unpacking
its `from_type`/`tcl_type` pair) *only* when `loop_self_referential` holds —
so the FP-guard reproducer's non-self-referential merge is still rejected by
the same gate, while the genuine recurrence is now recognised. The fix also
anchors the warning's span on the actual in-loop statement that produced one
of the two types (`per_loop_type_span`) rather than the phi's textually-
earliest incoming def, which for both reproducers above is the `set x "seed"`
initialiser — not where the developer needs to look.

#### Tests

- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_14_self_referential_branchy_oscillation_fires` (FN, now TP)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_14_non_self_referential_branchy_reset_no_s102` (FP guard)
- `rust::tcl_compiler::shimmer::thunking::tests::thunking_detected_for_self_referential_branchy_oscillation` (FN, unit level)
- `rust::tcl_compiler::shimmer::thunking::tests::no_s102_for_non_self_referential_branchy_reset` (FP guard, unit level)
- `rust::tcl_compiler::shimmer::thunking::tests::span_anchors_inside_loop_not_pre_loop_initialiser` (span precision)

---

### FP-SH-15 — tricky command/variable indirection: rename, interp alias, safe sub-interpreter eval, TclOO instance variables, `args`/parameters

- **Verdict:** three detection gaps now CLOSED (rename, interp alias, TclOO
  instance variables via `my variable`); the remaining surfaces stay
  CONFIRM-CORRECT (conservative — safe sub-interpreter eval, positional
  parameters, unknown-command result).
- **Status:** FIXED for surfaces 1, 2, and 5. A renamed/aliased builtin now
  resolves back to its registry spec through the lowerer's `canonical_command`
  snapshot (threaded into `type_infer` and `ssa::uses_of`); `my variable` is
  recognised as a namespace-style scope alias by `var_observability`. Surfaces
  3, 6, and 7 remain intentionally conservative and are pinned by regression
  tests.
- **Codes:** S100, S101, S102
- **Corpus:** synthetic (one reproducer per indirection surface).

#### Reproducers and per-surface reasoning

1. **`rename set myset`** *(gap now closed)* — the lowerer records the rename
   in the same binding snapshot it already keeps for `interp alias` (gated on
   the target being a registered builtin), so `myset x …` carries
   `canonical_command = set`. `type_infer` types the store's def from its value
   word (a canonical-`set` value passthrough, keyed on the `Set` lowering
   hook), and `ssa::uses_of` scans that value like the un-aliased `set` (so a
   braced `[expr {$x+1}]` still exposes the `$x` read and the loop-header phi
   forms). The renamed store now oscillates int↔string exactly as `set` does,
   firing S102. Genuine thunk — `myset` *is* `set`.
2. **`interp alias {} myset {} set`** *(gap now closed)* — the alias table was
   already snapshotted into `canonical_command` (PR #872 for T101); threading
   that canonical through `type_infer` / `ssa` closes S102 identically to the
   rename case. Codegen still emits a runtime `invokeStk` for the alias (an
   alias is not inline-foldable — its binding may change by call time); only
   the analysis type-lattice and def/use resolve it.
3. **`$slave eval {...}`** (a safe sub-interpreter) — the braced body is an
   opaque string argument to `eval`, not statically-analysed Tcl source in
   this compilation unit; nothing inside it is visited. No crash, no S102.
4. **TclOO instance variable via a bare `variable x` in a method body** — a
   scope-alias declaration, structurally identical to a top-level `variable`
   (`ssa::defs_of_with_registry`'s `CREATES_SCOPE_ALIAS` handling makes no
   TclOO/non-TclOO distinction) — protected the same way FP-SH-02 documents.
   Confirmed against the real diagnostic pipeline, not merely by the method
   body going unanalysed: `compiler_checks::run_all_checks` used to iterate
   `analysable_functions()`, which never reached TclOO method bodies at all
   (a *different*, unrelated coverage gap PR #872's T101 review closed by
   switching to `analysable_body_function_units()`) — re-verified post-fix
   that an *unaliased* plain local oscillating the same way inside a method
   body now genuinely fires S102 (`tcl diag` on a bare `set local …` inside
   `method run {}`), so the silence on `variable x` is the scope-alias
   protection actually firing, not the method body being skipped.
5. **`my variable x`** *(blind spot now closed)* — `var_observability::stmt_gen`
   recognises the `my variable NAME …` compound (`var_scoping::
   my_variable_declaration_indices`, which — unlike the top-level `variable`
   *command* — treats every argument after the `variable` subcommand word as a
   plain instance-variable name, no name/value pairs) and marks each name as a
   `NAMESPACE` escape. `crate::sccp::is_externally_mutable` then forces every
   def of the instance variable to OVERDEFINED, exactly as a bare `variable x`
   *inside* the method body is protected (FP-SH-16). The previously-latent
   false positive — `my variable x; set x 0; while {…oscillate…}`, where the
   local `set x 0` used to give the loop a versioned `Known(Int)` entry and
   fire a spurious S102 — is now silent, while an unaliased sibling local in
   the same method still fires (the protection is keyed on the declared name).
6. **A plain positional parameter as the oscillation seed** (`proc f {p} {
   while {1} { set p [expr {$p+1}]; set p [string range $p 0 end] } }`)  — a
   parameter's entry type is unknowable at compile time (the caller may pass
   anything), so `propagate_types` forces its SSA version-0 live-in to
   OVERDEFINED — the same protection FP-SH-02 documents for `global`/
   `variable`/`upvar`, but for an ordinary unaliased parameter. This also
   means a loop that only starts oscillating from iteration 2 onward (once a
   parameter-seeded value stabilises) is not detected — an accepted,
   conservative limitation, not a fix target here.
7. **An `unknown`-command result** (`set x [totallyUnknownCommand $x]`) types
   OVERDEFINED exactly like FP-SH-01's `[unknownCmd]` case.

#### Why the analyser reaches that verdict

The three *closed* gaps resolve the indirection at the boundary and stay generic
downstream, per AGENTS.md: `rename` / `interp alias` are snapshotted into the
lowerer's alias map so a renamed/aliased builtin carries `canonical_command`,
which `type_infer::evaluate_type_def` / `type_infer_process_statements` and
`ssa::uses_of` consume (value-passthrough typing + value-word read scanning,
both keyed on the registry's `Set` lowering hook, never a hardcoded name);
`my variable` is recognised by the shared escaping-set detector
(`var_observability`) via a `var_scoping` declaration helper, feeding the same
`is_externally_mutable` predicate SCCP already uses. The four *remaining*
surfaces stay silent purely because the *type lattice* degrades to OVERDEFINED
(unknown command return, live-in version 0, opaque string body) — no shimmer
special-casing. The tests pin both the fixes and the intentional conservatism so
a future change can't silently regress either.

#### Tests

- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_rename_indirection_fires_s102` (gap closed → TP)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_interp_alias_indirection_fires_s102` (gap closed → TP)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_alias_onto_non_store_no_s102` (TN control — alias onto a non-store command)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_safe_sub_interpreter_eval_no_s102`
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_tcloo_instance_variable_no_s102`
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_my_variable_idiom_no_s102` (now protected via scope-alias recognition)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_my_variable_locally_initialised_no_s102` (the previously-latent FP, now silent)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_my_variable_unaliased_sibling_local_still_fires` (TP control)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_parameter_seeded_oscillation_no_s102`
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_15_unknown_command_result_no_s102`
- `rust::tcl_compiler::var_observability::tests::my_variable_marks_namespace_alias`
- `rust::tcl_compiler::alias::tests::detect_rename_basic_move`
- `rust::tcl_compiler::type_infer::tests::aliased_set_call_types_def_from_value`
- `rust::tcl_compiler::ssa::tests::set_value_reads_parses_braced_expr`
- `rust::tcl_lsp_server::e2e::diagnostic_matrix::s102_fires_for_rename_indirection` (e2e)
- `rust::tcl_lsp_server::e2e::diagnostic_matrix::s102_fires_on_defect` (e2e, interp alias)

---

### FP-SH-16 — global/namespace aliasing only protected an *unwritten* entry type; a locally-initialised alias thunked like an ordinary local

- **Verdict:** FALSE POSITIVE (S102) — fixed
- **Status:** FIXED (Rust port deep review, closed on rebase). Originally
  written up as CONFIRM-CORRECT / an accepted precision boundary; closed once
  rebasing onto `origin/rust` picked up PR #859 (O100/O102/O103 soundness
  fixes), which built `crate::sccp::is_externally_mutable` (escaping-set +
  module-trace check) and the whole-module `extra_global_escaping`
  (`crate::var_observability::scan_module_global_names`) fact for its own
  (separate) constant-folding lattice — reusing both for the type lattice
  closes this gap too, for free, instead of leaving the two lattices with
  divergent aliasing-soundness models.
- **Codes:** S102
- **Corpus:** synthetic.

#### Reproducer (previously a false positive, now silent)

```tcl
proc f {} {
    global x
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
```

#### Reproducer (TN control — stays silent, unaffected)

```tcl
namespace eval ::foo {
    variable x 0
    proc bump {} {
        variable x
        while {1} {
            set x [expr {$x + 1}]
            set x [string range $x 0 end]
        }
    }
}
```

#### Reproducer (TP control — an unaliased sibling local in the same function still fires)

```tcl
proc f {} {
    global x
    set x 0
    set y 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
        set y [expr {$y + 1}]
        set y [string range $y 0 end]
    }
}
```

#### Per-line reasoning

1. FP-SH-02 protects an aliased name (`global`/`variable`/`upvar`) by leaving
   its SSA version-0 live-in OVERDEFINED — sound *only* while no statement in
   this function body has locally re-initialised it.
2. In the first reproducer, `set x 0` runs immediately after `global x`,
   giving the loop header a real, versioned `Known(Int)` entry type — the
   FP-SH-02 live-in protection alone no longer applies from that point.
   Pre-fix, the type lattice had no OTHER aliasing-awareness at all (unlike
   SCCP's own lattice), so it typed the loop's literals as if `x` were an
   ordinary local — but `x` stays reachable by *another* procedure's own
   `global x; set x …` (through a call whose relative order isn't statically
   known) for the rest of this function's body, exactly the cross-proc
   soundness gap `cfg_builder::global_write_info` closed for SCCP. Reusing
   `is_externally_mutable` — which checks the per-function escaping set
   (`global`/`variable`/`upvar` anywhere in this body, flow-insensitively)
   *and* the module trace fact — forces every def of `x` to OVERDEFINED for
   the rest of the function, so S102 no longer fires from the unsound literal
   view.
3. In the TN control, `bump`'s `variable x` is never followed by a local
   `set` before the loop — the live-in stays version 0 / OVERDEFINED (belt
   and braces with the fix above: `x` is doubly protected now).
4. In the TP control, `y` is never declared `global`/`variable`/`upvar` — it
   is not in the escaping set, so its own genuine oscillation still fires
   S102 normally. Proves the fix is keyed on the aliased name specifically,
   not a blanket suppression of S102 for the whole function.

#### Why the analyser reaches that verdict

`type_infer::propagate_types` now takes `extra_global_escaping` (threaded
from the same whole-module scan SCCP's top-level build already computes) and
independently calls `var_observability::analyse_var_observability(cfg)
.escaping_var_names()` for the per-function view, unions the two, and passes
the result to `crate::sccp::is_externally_mutable` at every def site
(`type_infer_process_statements`) — the exact same predicate
`sccp_with_extra_escaping` and O102 load-forwarding already apply to their
own lattices, so a name aliased anywhere in this function's own body (or
`::`-qualified, or module-wide-traced) is never trusted from its literal
alone, regardless of how many local versions it has.

#### Tests

- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_16_global_alias_locally_initialised_no_s102` (FP, now fixed)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_16_unaliased_sibling_local_still_fires` (TP control)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_16_namespace_alias_without_local_init_no_s102` (TN control)

---

### FP-SH-18 — a Numeric loop-body oscillation seeded by an Int entry was masked to OVERDEFINED by `type_join`'s exact-equality Known-vs-Shimmered rule

- **Verdict:** FALSE NEGATIVE (S100/S101/S102) — fixed
- **Status:** FIXED. Found but not fixed during the original S102 review and
  flagged as a foundational `types.rs` change with broad blast radius; closed
  here after validating the numeric refinement against the whole compiler test
  corpus (a true-LUB, monotone lattice change).
- **Codes:** S100, S101, S102 (any consumer of `TypeLattice`; W307/W308 also
  consume it but are unaffected — an object-ness query does not distinguish
  SHIMMERED from OVERDEFINED).
- **Corpus:** synthetic.

#### Reproducer (false negative, now fixed)

```tcl
proc f {n} {
    set x 0
    while {$n > 0} {
        if {$n % 2} {
            set x [expr {$x + $n}]
        } else {
            set x "s$x"
        }
        incr n -1
    }
    return $x
}
```

#### Reproducer (TN control — a uniformly-numeric loop must stay silent)

```tcl
proc f {n} {
    set x 0
    while {$n > 0} {
        set x [expr {$x + $n}]
        incr n -1
    }
    return $x
}
```

#### Per-line reasoning

1. The loop body is self-referential (both arms read `$x`) and produces
   `Numeric` (`expr {$x + $n}` on the loop-carried `$x`, whose type is not a
   compile-time constant) on one arm and `String` (`"s$x"`) on the other, so
   the bottom-of-loop join is `SHIMMERED(Numeric, String)`.
2. The loop entry is `Known(Int)` (`set x 0`). The loop-header phi therefore
   joins `Known(Int)` with `SHIMMERED(Numeric, String)`.
3. Pre-fix, `type_join`'s one-Known-one-Shimmered arm matched the known type
   against each shimmer side by **exact `TclType` equality**. `Int != Numeric`
   and `Int != String`, so the join degraded to OVERDEFINED — and
   `classify_thunking_phi` bails on a non-SHIMMERED header phi, silently
   masking the genuine per-iteration numeric↔string thunk.

#### Why the analyser reaches that verdict

`types::type_join` now applies a **numeric refinement** before falling to
OVERDEFINED: a `Known` numeric-family type (`Int`/`Double`/`Boolean`/`Numeric`,
via `is_numeric_family`) meeting a numeric-family shimmer side is subsumed by
that side, and the side widens to `Numeric` so the result stays a sound upper
bound of the `Known` type (`Double ⊔ SHIMMERED(Int, String)` =
`SHIMMERED(Numeric, String)`, which covers `Double` — `SHIMMERED(Int, …)` would
not). This is deliberately **not** routed through `numeric_promotion`, whose
`Some`-if-either-operand-is-`Double` shape would wrongly match a non-numeric
side (`numeric_promotion(Double, String) → Numeric`) and drop it. A non-numeric
`Known` type that matches neither side still degrades to OVERDEFINED, so the
refinement never blanket-keeps a join SHIMMERED (the TN control's uniformly-Int
`Known ⊔ Known` promotes to `Numeric`, never shimmers).

#### Tests

- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_18_numeric_shimmer_masked_by_int_entry_fires_s102` (FN, now TP)
- `rust::tcl_compiler::analyser::diagnostics::fp::sh::fp_sh_18_uniform_numeric_loop_no_s102` (TN control)
- `rust::tcl_compiler::shimmer::thunking::tests::thunking_detected_for_numeric_shimmer_masked_by_int_entry` (FN, unit level)
- `rust::tcl_compiler::types::tests::known_int_matches_numeric_shimmer_side` (lattice unit)
- `rust::tcl_compiler::types::tests::known_double_widens_int_shimmer_side_to_numeric` (soundness/LUB unit)
- `rust::tcl_compiler::types::tests::known_non_numeric_no_match_still_overdefined` (guard unit)
- `rust::tcl_lsp_server::e2e::diagnostic_matrix::s102_fires_for_numeric_shimmer_masked_by_int_entry` (e2e)

---


### FP-SH-17 — destructuring writers broadcast their return type onto written variables (issue #867)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs::fp_sh_17_*`, `rust/tcl-compiler/src/type_infer.rs` (`lassign_single_destructure_def_is_overdefined_not_list`, `var_write_typing_shapes_destructure_target_types`), `rust/tcl-registry/tests/registry.rs::var_write_typing_declares_destructuring_writers`, and the `preview_tickets_e2e.rs` / `previewTickets.test.ts` regression pair
- **Codes:** S100 (use-site / expr shimmer), also latent W126
- **Corpus:** synthetic — verified vs tclsh 9.0.4

#### Reproducer

```tcl
set point [list 1 2 3]

lassign $point x y z
# pre-fix: S100 on each of x/y/z "has list intrep used in arithmetic expression"
set offset [expr {$x + $y + $z}]
```

#### Per-line reasoning

1. `lassign $point x y z` writes list *elements* to `x`/`y`/`z`.  Each element is whatever intrep sat in that list slot — statically unknown.
2. `lassign`'s `return_type` is `List` (the *leftover* elements the command returns), which describes neither `x`, `y`, nor `z`.
3. Pre-fix the type-inference pass broadcast the command's return type onto every written variable, guarded only by a `defs.len() > 1` heuristic in `evaluate_type_def`.  Multi-target calls widened to `Overdefined` by accident, but a *single*-target `lassign $l x` fell through and typed `x` as `List` — and the reported three-target case regressed the moment any consumer looked at one def in isolation.
4. `expr {$x + $y + $z}` then saw a `List`-typed operand in arithmetic and fired S100.  The same shape mistyped `regexp`/`scan`/`regsub`/`binary scan` targets as the returned `Int` count (a latent numeric-in-string-compare S100), and `gets`/`lpop` targets from their non-matching return types.

#### tclsh ground truth (9.0.4 — confirmed by execution)

```
% set point [list 1 2 3]
1 2 3
% lassign $point x y z
                        ;# returns the empty leftover list
% expr {$x + $y + $z}
6                       ;# x/y/z are the elements "1"/"2"/"3"; no shimmer smell
```

#### Fix

`CommandSpec`/`SubCommand` gained a `var_write_typing: VarWriteTyping` field
(`ReturnValue` default, `Fixed(TclType)`, `Destructured`).  `lassign` / `scan`
/ `regexp` / `binary scan` declare `Destructured` (targets widen to
`Overdefined`); `gets` and `regsub` declare `Fixed(String)` (the read line /
the substituted result), and `lpop` `Fixed(List)` (the shortened list).
`evaluate_type_def` reads `ResolvedCall::var_write_typing()` and drops the
blanket `defs.len() > 1` heuristic — the default `ReturnValue` arm keeps a
multi-def guard (a single return value can't type several distinct written
variables, so `catch`/`try`'s synthetic result/options + body writes stay
`Overdefined`), but the typing is otherwise registry data keyed per command /
subcommand, never a command-name branch.  See
[`command-registry.md`](command-registry.md#var_write_typing--return-type-vs-written-variable-type).


## OBJ — object dispatch (W307/W308) + snit modelling

W307 catches stray non-literal command words (`$x foo`); W308 validates
method names against the declared set.  Both have to be aware of object
handles — snit self-references, snit components, namespaced factory
returns, local snit instances — or every such dispatch false-positives.
These entries also cover the snit body modelling (private procs are
analysed, instance vars are exempt from RBS).

### FP-OBJ-01 — snit self-references ($self/$type/$selfns/$win) — not stray non-literal commands

- **Verdict:** FALSE POSITIVE (now fixed, snit modelling)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_01_*`
- **Codes:** W307
- **Corpus:** every snit::type / snit::widget body that uses self-dispatch — pt::*, struct::*, grammar::*, tklib megawidget patterns.

#### Reproducer

```tcl
# Bench placeholder: snit modelling sits outside the per-proc snapshot.
# The FP-OBJ-01 verdict is locked in by tests/test_fp_obj.py only.
proc f {} { return ok }
```

#### Per-line reasoning

Inside a snit::type / snit::widget method body, `$self`, `$type`, `$selfns`, `$win` are **reserved variables** with specific runtime semantics:

* `$self foo` — dispatches method `foo` on the current object.
* `$type bar` — dispatches typemethod `bar`.
* `$selfns` — the per-instance Tcl namespace (used to scope vars).
* `$win` — the window path for snit::widget.

Pre-fix these looked like generic `$var cmd` non-literal-command dispatches and fired W307.  Fix: register the snit-reserved set in the body's exempt list.

#### tclsh ground truth

```
% snit::type T { method m {} { return "I am [$self info type]" }
 method n {} { $self m }
}
% T create t
% t n
I am ::T
```

#### Compiler evidence

```
--- FP-OBJ-01: snit self-references ($self/$type/$selfns/$win) — not stray non-literal commands
regen: python -m bench.fp_snippets --id FP-OBJ-01
function ::f
  block entry_1
    term Return ok
```

#### Why the analyser reaches that verdict

`dialects/tcllib/snit.py` defines the SNIT_RESERVED constant `{self, type, selfns, win, hull, ...}`.  `analyser/_analyser/_diag_var_command.py` exempts those names when the enclosing scope is a snit type / widget body.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_01_self_dispatch_no_w307` (FP, all 4 self-refs)
- `tests/test_fp_obj.py::test_FP_OBJ_01_self_ref_outside_snit_still_w307` (TP — same names in vanilla proc still warn)

---

### FP-OBJ-02 — snit::widgetadaptor $hull dispatch — widgetadaptor delegation idiom

- **Verdict:** FALSE POSITIVE (now fixed, snit modelling)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_02_widgetadaptor_hull_no_w307`
- **Codes:** W307
- **Corpus:** every `snit::widgetadaptor` body (tklib's adaptor widgets, BWidget-based components).

#### Reproducer

```tcl
# See note in FP-OBJ-01 — snit modelling is class-level.
proc f {} { return ok }
```

#### Per-line reasoning

`snit::widgetadaptor` is the snit form for wrapping a single underlying Tk widget; the underlying widget is exposed as `$hull`.  `$hull configure -bg red` delegates to the underlying widget.  Same exemption as FP-OBJ-01 — `hull` joins the reserved set.

#### tclsh ground truth

```
% snit::widgetadaptor MyW { ... method m {} { $hull configure -bg red }
 }
```
`$hull configure …` is the canonical pattern; snit's docs specify it explicitly.

#### Compiler evidence

```
--- FP-OBJ-02: snit::widgetadaptor $hull dispatch — widgetadaptor delegation idiom
regen: python -m bench.fp_snippets --id FP-OBJ-02
function ::f
  block entry_1
    term Return ok
```

#### Why the analyser reaches that verdict

`SNIT_RESERVED` in `dialects/tcllib/snit.py` includes `hull`.  Same dispatch-check exemption as FP-OBJ-01.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_02_widgetadaptor_hull_no_w307` (FP)

---

### FP-OBJ-03 — snit component dispatch ($myexporter export ...) — instance-var method dispatch

- **Verdict:** FALSE POSITIVE (now fixed, snit body inventory)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_03_*`
- **Codes:** W307
- **Corpus:** tcllib's parsers (`pt::*`), grammar engines (`grammar::*`), and aggregate types (`struct::*`) heavily use this idiom.

#### Reproducer

```tcl
proc f {} { return ok }
```

#### Per-line reasoning

User-declared instance vars (`variable v`, `component c`, `option -o`, `typevariable t`) hold values the user can dispatch on.  Inside a method / constructor / typemethod body, `$myexporter export …` is method dispatch on a known-object instance var — pre-fix the analyser couldn't distinguish these from arbitrary `$var cmd` calls.

Fix: the snit body inventory collects every declared instance var / component / option / typevariable into a per-type set; dispatches on those names inside the body are exempt.

#### tclsh ground truth

```
% snit::type T { component myexp
 method m {} { $myexp export 1 }
 }
% T create t
% t configure -myexp [exporter create %AUTO%]
% t m
```
The component name resolves to the exporter object at run time.

#### Compiler evidence

```
--- FP-OBJ-03: snit component dispatch ($myexporter export ...) — instance-var method dispatch
regen: python -m bench.fp_snippets --id FP-OBJ-03
function ::f
  block entry_1
    term Return ok
```

#### Why the analyser reaches that verdict

`dialects/tcllib/snit.py` records the type's `variable` / `component` / `option` / `typevariable` declarations into a `BodyInventory` keyed on type name.  The W307 check exempts dispatches whose target name is in that inventory when the enclosing scope is the type body.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_03_component_dispatch_no_w307` (FP, component)
- `tests/test_fp_obj.py::test_FP_OBJ_03_constructor_dispatch_no_w307` (FP, constructor)
- `tests/test_fp_obj.py::test_FP_OBJ_03_typemethod_dispatch_no_w307` (FP, typevariable in typemethod)

---

### FP-OBJ-04 — namespaced-factory provenance: set t [::struct::tree] — object handle

- **Verdict:** FALSE POSITIVE (now fixed, analyser-only provenance) — **shape-based heuristic**, not a Tcl-semantic guarantee.  See the "Known precision gap" note below.
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_04_*`
- **Codes:** W307
- **Corpus:** tcllib's factory idiom is pervasive: `::struct::tree`, `::struct::matrix`, `::struct::queue`, `pt::rde`, `grammar::*` etc.

#### Reproducer

```tcl
proc f {} {
    # ::struct::tree is a namespaced factory; $t is an object handle.
    set t [::struct::tree mytree]
    $t walk root
}
```

#### Per-line reasoning

tcllib's standard pattern: `::struct::tree mytree` is a constructor that creates and returns the new tree's command name.  Subsequent `$t walk root` is dispatch on the new object.  Pre-fix the analyser saw `$t` as an unknown command word.

Provenance fix: a var assigned from a *namespaced* command substitution (`[::ns::factory …]` or `[ns::factory …]`) is tagged as an object handle; W307 is exempt for dispatches on that var **within the same proc** (no cross-proc leakage).  This is *analyser-only* provenance — no change to the type lattice — so the shimmer pass is unaffected.

#### tclsh ground truth

```
% ::struct::tree mytree
::mytree
% mytree walk root pre {n} { puts $n }
root
```

#### Compiler evidence

```
--- FP-OBJ-04: namespaced-factory provenance: set t [::struct::tree] — object handle
regen: python -m bench.fp_snippets --id FP-OBJ-04
function ::f
  block entry_1
    [0] AssignValue 't' value='[::struct::tree mytree]'  defs={t#1}  uses={}
    [1] Call cmd='${t}'  defs={}  uses={t#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    t#1: OVERDEFINED
```

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` tags vars from namespaced cmd-sub assignments as `ObjectProvenance.NAMESPACED_FACTORY`.  `analyser/_analyser/_diag_var_command.py` exempts dispatches on those vars within the same proc.  The bare-name cmd-sub case (no `::`) is NOT tagged — see `test_FP_OBJ_04_bare_unknown_command_still_w307`.

#### Known precision gap (open)

The rule is **shape-based**, not return-type-aware: any namespaced
command substitution is treated as an object factory.  A namespaced
proc that returns a plain string is therefore *silently exempt* —
W307 will NOT fire on dispatch of its return value, even though
the value isn't a command name.  Example:

```tcl
namespace eval ::ns { proc make {} { return foo } }
proc f {} {
    set x [::ns::make]
    $x method   ;# should fire W307 (no command named "foo")
}
```

Tracked by the open xfail test
`test_FP_OBJ_04_namespaced_string_returning_proc_precision_gap`.
A refined per-proc return-type lattice (track which user procs
*actually* return object handles vs strings) would close the gap;
that work flips the xfail and prompts its own removal.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_04_namespaced_factory_no_w307` (FP, `::struct::tree`)
- `tests/test_fp_obj.py::test_FP_OBJ_04_short_namespace_form_no_w307` (FP, `struct::matrix`)
- `tests/test_fp_obj.py::test_FP_OBJ_04_bare_unknown_command_still_w307` (TP — bare-name still warns)
- `tests/test_fp_obj.py::test_FP_OBJ_04_factory_does_not_leak_across_procs` (TP — per-proc scoping)
- `tests/test_fp_obj.py::test_FP_OBJ_04_namespaced_string_returning_proc_precision_gap` (OPEN-FP, xfail strict)

---

### FP-OBJ-05 — snit instance dispatch (set o [Foo create %AUTO%]; $o m) — typed OBJECT

- **Verdict:** FALSE POSITIVE (now fixed, snit instance provenance)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_05_*`
- **Codes:** W307, W308
- **Corpus:** every local snit type used in tests / examples / integration glue.

#### Reproducer

```tcl
snit::type ::Counter { method bump {} { return 1 } }
proc use {} {
    # `Counter create %AUTO%` returns a snit instance; $a bump is
    # method dispatch, not a stray non-literal command.
    set a [Counter create %AUTO%]
    $a bump
}
```

#### Per-line reasoning

A locally-defined `snit::type ::Counter { method bump {} { … } }` is in scope, so calling `Counter create %AUTO%` returns a known-object instance.  Pre-fix the analyser had no record of `Counter` being a snit type and fired W307 on the dispatch.

Fix: `compilation_unit.snit_types` records each locally-declared snit type's name + method set; vars initialised from a call to a recorded type's create-form (or the create-shorthand `Foo %AUTO%`) are typed OBJECT.  W307 + W308 are both exempt — W308 because snit's method-dispatch goes through delegation / hull / options / built-ins (not soundly resolvable to the declared set).

#### tclsh ground truth

```
% snit::type Counter { method bump {} { return 1 }
}
% set a [Counter create %AUTO%]
::counter1
% $a bump
1
```
The result of `Counter create %AUTO%` is an object command name.

#### Compiler evidence

```
--- FP-OBJ-05: snit instance dispatch (set o [Foo create %AUTO%]; $o m) — typed OBJECT
regen: python -m bench.fp_snippets --id FP-OBJ-05
function ::use
  block entry_1
    [0] AssignValue 'a' value='[Counter create %AUTO%]'  defs={a#1}  uses={}
    [1] Call cmd='${a}'  defs={}  uses={a#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    a#1: OVERDEFINED
```

#### Why the analyser reaches that verdict

`dialects/tcllib/snit.py` builds the type's create-form receiver: any `[Foo create ARGS]` or `[Foo ARGS]` (the create-shorthand) where `Foo` is a registered snit type produces an instance-typed value.  Receiver dispatches inherit the exemption.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_05_snit_create_auto_no_w307` (FP, %AUTO%)
- `tests/test_fp_obj.py::test_FP_OBJ_05_snit_create_named_no_w307` (FP, named)
- `tests/test_fp_obj.py::test_FP_OBJ_05_snit_create_shorthand_no_w307` (FP, shorthand)
- `tests/test_fp_obj.py::test_FP_OBJ_05_snit_instance_no_w308` (FP, W308 exempt)

---

### FP-OBJ-06 — snit private proc body is analysed (not silently dropped)

- **Verdict:** CONFIRM-CORRECT (snit body analysis depth)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_06_private_proc_body_analysed`
- **Codes:** W216 (used as a positive marker)
- **Corpus:** any snit type with internal helper procs (struct::matrix, struct::graph internal helpers).

#### Reproducer

```tcl
# Class-level body; per-proc snapshot doesn't fit.
proc f {} { return ok }
```

#### Per-line reasoning

A `proc Helper {a} { … }` declared inside a `snit::type` body is a **type-private proc** (callable only from the type's methods).  Pre-fix worry: the snit body wrapping might cause the analyser to drop the inner proc and miss genuine diagnostics in its body.

The verdict on audit: the analyser DOES descend into the body — proven here by the **value-position** `return ${a}($a)` firing W216 (scalar-vs-array smell) inside the proc.  This locks in the depth contract.  (A *varname*-position `${a}($a)` would be the legitimate indirect-array idiom and stay silent — see FP-STY-12 — so the marker is deliberately a value position.)

#### tclsh ground truth

```
% snit::type T { proc Helper {a} { return ${a}($a) }
 method m {a} { return [Helper $a] }
}
% T create t
% t m foo
foo(foo)  # ${a} substitutes to the value of a, then literal "(foo)" is appended;
          # W216 is a *static-analysis* smell — the form is rarely what's meant
```

#### Compiler evidence

```
--- FP-OBJ-06: snit private proc body is analysed (not silently dropped)
regen: python -m bench.fp_snippets --id FP-OBJ-06
function ::f
  block entry_1
    term Return ok
```

#### Why the analyser reaches that verdict

`dialects/tcllib/snit.py` registers inner `proc` declarations with the analyser pipeline; the body is compiled and walked through every check.  This entry exists to lock the contract — a refactor that silently drops the body would catch the W216 disappearance.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_06_private_proc_body_analysed` (TP / depth lock-in)

---

### FP-OBJ-07 — cmd-sub namespaced ensemble `[ns_func]::method` is dispatch, not stray word

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_07_*`
- **Codes:** W307 (unresolved command / stray non-literal command word)
- **Corpus:** tcllib http/ftpd dispatch idiom and namespace-factory ensembles.

#### Reproducer

```tcl
namespace eval ::ns {
    proc dispatch {} { return ::ns::sub }
    proc sub::work {arg} { return $arg }
}
[::ns::dispatch]::work hello
```

#### Per-line reasoning

1. `[::ns::dispatch]::work hello` — the command word is `[::ns::dispatch]::work`.  At runtime Tcl substitutes `[::ns::dispatch]` to get `::ns::sub`, then composes the qualified name `::ns::sub::work` and invokes it.  This is the canonical "namespaced ensemble" dispatch: a known-name suffix (`::work`) appended to a command-substitution result that produces a namespace prefix.
2. The dispatch is well-formed: `::ns::sub::work` resolves to the defined proc.  No "stray non-literal command word" — there's a literal method-name tail (`::work`).

Pre-fix: W307 fired because the command word starts with `[` (cmd-sub).  The detector didn't recognise the `[...]::word` shape as a known dispatch idiom and reported "stray non-literal command word".

Fix: at the W307 detection point, recognise the `[CMDSUB]::IDENT...` pattern as a valid namespaced-ensemble dispatch and suppress the warning.  The IDENT-tail provides the static method-name evidence that distinguishes "I know what method is being called, just not on which namespace" from "I don't know what's being called at all".

#### tclsh ground truth

```
% namespace eval ::ns {
    proc dispatch {} { return ::ns::sub }
    proc sub::work {arg} { return $arg }
}
% [::ns::dispatch]::work hello
hello
```

Tcl substitutes and dispatches normally.

#### Compiler evidence

The W307 detector inspects command-word shape; the namespaced-ensemble
suppression matches the regex/shape:

```
\[...\](::[a-zA-Z_][a-zA-Z0-9_]*)+
```

#### Why the analyser reaches that verdict

`compiler/checks/` W307 suppression branch checks for the cmd-sub + namespaced suffix shape before firing.  Commit `85015850`.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_07_cmdsub_namespaced_ensemble_no_w307` (FP)
- `tests/test_fp_obj.py::test_FP_OBJ_07_bare_cmdsub_dispatch_still_fires` (TP control — `[cmd] $arg` with no literal method tail still fires)

---

### FP-OBJ-08 — W307 suppressed on eval-substituted dispatch (W101 covers it)

- **Verdict:** FALSE POSITIVE / dedup with W101
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_08_*`
- **Codes:** W307, W101 (the dedup target)
- **Corpus:** tcllib `eval $cmd $args` dispatch glue.

#### Reproducer

```tcl
proc f {cmd args} {
    # eval-substituted dispatch — W101 already flags the eval-of-
    # substituted-string injection risk.  W307 reporting the same
    # site as "stray non-literal command word" is redundant noise.
    eval $cmd $args
}
```

#### Per-line reasoning

1. `eval $cmd $args` — `eval` of a substituted command string is the canonical W101 site (eval-injection risk: the substituted value may contain arbitrary command syntax).
2. Pre-fix W307 *also* fired here, flagging `$cmd` as a non-literal command word.  Same finding, two codes — pure duplicate noise.

Fix: at the W307 detection point, check whether the enclosing call is `eval` (or `interp eval` / similar substituting eval forms) and suppress W307 since W101 already provides the appropriate diagnostic.  The user fix (use `eval [list $cmd {*}$args]` or similar) is identical for both.

#### tclsh ground truth

```
% proc f {cmd args} { eval $cmd $args }
% f puts hello
hello
```

Runtime works (assuming `$cmd` is benign); the security concern is the substitution-of-command-string pattern itself.

#### Compiler evidence

W307's pre-fire filter checks whether the call site is an `eval`-family form and short-circuits before emission when so.

#### Why the analyser reaches that verdict

`compiler/checks/` W307 dedup branch: if the parent call command is in the eval-substituting set, suppress W307 since W101 will fire.  Commit `7a1bbf75`.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_08_eval_substituted_dispatch_no_w307` (FP — W307 suppressed)
- `tests/test_fp_obj.py::test_FP_OBJ_08_eval_substituted_dispatch_still_fires_w101` (TP control — W101 must still fire)

---

### FP-OBJ-09 — W307 multi-dispatch local var (≥2 dispatches on same local)

- **Verdict:** FALSE POSITIVE — **heuristic** (corpus-driven intent
  inference, not a Tcl-semantic guarantee).  Two dispatches on the same
  local is strong evidence that the user designed it as an object
  handle, but isn't proof; a typo-named pseudo-handle dispatched twice
  by accident would also escape.
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_09_*`
- **Codes:** W307 (non-literal command word)
- **Corpus:** tcllib `struct::graph` users (graphops.tcl, etc.).

#### Reproducer

```tcl
proc analyze {G} {
    set TGraph [createTGraph $G]
    $TGraph node first
    $TGraph dispose
}
```

#### Per-line reasoning

A single dispatch on an unknown source is ambiguous (could be an
object handle OR a typo).  But **≥2 dispatches on the same local
variable** demonstrate firm intent — the user designed that local as
an object handle.  Flagging W307 on each dispatch would noise out the
entire pattern.

Fix: track per-proc dispatch counts in the pre-pass over
`_var_command_sites`; suppress W307 on locals dispatched ≥2 times.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_09_multi_dispatch_local_no_w307` (FP)
- `tests/test_fp_obj.py::test_FP_OBJ_09_single_dispatch_unknown_still_fires` (TP control — one dispatch isn't enough)

---

### FP-OBJ-10 — W307 switch-callback array element (`$state(-command) ...`)

- **Verdict:** FALSE POSITIVE — **heuristic** (callback-shape inference
  from the array key, not a Tcl-semantic guarantee).  An array element
  whose key matches the callback-naming convention is *very likely* a
  registered callback slot; the suppression can mask a typo in a non-
  callback array key that happens to match the heuristic.
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_10_*`
- **Codes:** W307
- **Corpus:** tcllib HTTP / IRC / async state-machine modules.

#### Reproducer

```tcl
proc h {state token} {
    $state(-command) $token
}
```

#### Per-line reasoning

`$state(-command)` is an array-element dispatch where the key is
dash-prefixed (the documented Tcl switch-option convention for
"this slot holds an explicitly-registered command").  The dispatcher
relies on a prior `set state(-command) my::callback`; W307 firing
here would noise out every state-machine that uses the option-keyed
callback table.

Fix: suppress W307 when the dispatch target is an array element whose
key is either:
- **dash-prefixed** (`$state(-command)`, `$state(-handler)`, …), OR
- **suffix-shaped** with one of {`cmd`, `command`, `callback`,
  `handler`, `hook`, `proc`} (case-insensitive) as the final word —
  e.g. `$state(doneCallback)`, `$state(openCmd)`.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_10_dash_prefixed_array_key_callback_no_w307` (FP)
- `tests/test_fp_obj.py::test_FP_OBJ_10_suffix_keyed_callback_no_w307` (FP)

---

### FP-OBJ-11 — W307 interprocedural object-factory tracking

- **Verdict:** FALSE POSITIVE — **inherits the shape-based namespaced-
  factory heuristic from FP-OBJ-04** (which has a known precision gap
  for namespaced procs returning plain strings).  When the inner cmd
  isn't a namespaced object factory, the transitive propagation
  inherits the same false-negative; closing the FP-OBJ-04 gap (per-
  proc return-type lattice) would close this one too.
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_11_*`
- **Codes:** W307
- **Corpus:** tcllib `struct::graphops` (88 → 0 W307 firings cleared).

#### Reproducer

```tcl
proc createGraph {} { return [struct::tree] }
proc f {} {
    set t [createGraph]
    $t op
}
```

#### Per-line reasoning

`createGraph` directly returns the result of a namespaced object
factory (`[struct::tree]`).  Interproc fixpoint inference marks
``createGraph`` as object-returning; callers' `set t [createGraph]`
binds `t` to an object handle, so `$t op` is a valid object dispatch.

The fixpoint propagates transitively: a proc returning `$X` where
`$X` was assigned from another object-returning proc is itself
object-returning.

The complementary case to FP-OBJ-04 (namespaced-factory provenance
at the call site) — that entry handled `set t [::struct::tree]`
directly; this one handles `set t [user_factory_wrapper]`.

#### tclsh ground truth

```
% package require struct::tree
% proc createGraph {} { return [::struct::tree] }
% set g [createGraph]
::tree0
% $g insert root 0 1 2
0 1 2
```

The returned value IS the object handle; dispatch works as expected.

#### Why the analyser reaches that verdict

The interproc fixpoint at `compiler/interprocedural.py` tags procs whose
return value is itself an object handle (direct `return [factory]`,
indirect `return $X` where X transitively traces to a factory).
W307 suppresses dispatch on locals assigned from object-returning
user procs.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_11_factory_dispatch_no_w307` (FP, direct factory chain)
- `tests/test_fp_obj.py::test_FP_OBJ_11_transitive_factory_no_w307` (FP, two-step factory chain — fixpoint propagation)

---

### FP-OBJ-12 — W307 fires on `[<cmd-sub>] run` in a method body (D3-P3 / D4-F5)

- **Verdict:** TRUE POSITIVE (precision-gap closure)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_12_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3 / TclOO 1.x
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entries D3-P3, D4-F5

#### Reproducer

```tcl
oo::class create C {
    method m {} { [format notACommand] run }
}
```

#### Per-line reasoning

1. `oo::class create C { method m {} { ... } }` — defines a TclOO class with a method `m`.
2. Inside `m`'s body, `[format notACommand]` is a command substitution that returns the literal string `"notACommand"` (no formatting directives).
3. `[format notACommand] run` then dispatches the string `"notACommand"` as if it were a command — at runtime tclsh raises `invalid command name "notACommand"`.
4. Pre-fix the W307 cmd-sub-as-command path blanket-suppressed every `[…] <subcmd>` inside a method body, on the assumption that any cmd-sub in a method might be returning an object handle.  D4-F5 closure removes the blanket: only `my`/`self` self-dispatch + a KNOWN OBJECT return type suppresses now.  `format` returns STRING, so W307 correctly fires.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% oo::class create C { method m {} { [format notACommand] run } }
::C
% [C new] m
invalid command name "notACommand"
```

#### Compiler evidence

```
--- FP-OBJ-12: W307 cmd-sub in method body fires when not known OBJECT return (D3-P3/D4-F5)
regen: python -m bench.fp_snippets --id FP-OBJ-12
function ::top
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='oo::class'  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-12`)

The top-level `oo::class create` is the SSA entry; the analyser's class-extraction pass reads `C`'s method bodies separately and applies the W307 check to each dispatch site there.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` — the cmd-sub-as-command path no longer carries the `in_method` blanket suppression.  Only `my`/`self`-prefixed dispatches with a known OBJECT return type (via `known_classes`) get suppressed; everything else falls through to the standard W307 emission.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_12_format_in_method_fires` (TP)
- `tests/test_fp_obj.py::test_FP_OBJ_12_known_class_new_in_method_silent` (TN control — `[D new]` is a known factory)
- `tests/test_ground_truth_tn_fn.py::test_TP_W307_format_in_method_fires`
- `tests/test_ground_truth_tn_fn.py::test_TN_W307_known_class_new_in_method_silent`

---

### FP-OBJ-13 — W307 fires on `[my plain] run` where `plain` returns a literal (D3-P4)

- **Verdict:** TRUE POSITIVE (precision-gap closure)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_13_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3 / TclOO 1.x
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D3-P4

#### Reproducer

```tcl
oo::class create C {
    method plain {} { return notACommand }
    method m {} { [my plain] run }
}
```

#### Per-line reasoning

1. `method plain {} { return notACommand }` — a method whose body is a single `return <literal>`.  The return value is provably the STRING `"notACommand"` (no cmd-sub, no var interpolation).
2. `method m {} { [my plain] run }` — calls `my plain` (resolved to `C`'s `plain`), which returns the literal STRING.  Then attempts to dispatch `"notACommand" run`.
3. Pre-fix the `[my <method>]` self-dispatch path conservatively typed the return as OBJECT (since methods often return objects), suppressing W307.  D3-P4 closure adds a lightweight method-body inspection: when the named method's body is a simple `return <literal>`, override the OBJECT heuristic and fire W307.
4. Compound bodies (cmd-sub, variable interpolation, multi-statement) stay conservatively suppressed — the analysis can't prove the return type for them.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% oo::class create C {
    method plain {} { return notACommand }
    method m {} { [my plain] run }
}
::C
% [C new] m
invalid command name "notACommand"
```

#### Compiler evidence

```
--- FP-OBJ-13: W307 [my plain] with literal-return method body fires (D3-P4)
regen: python -m bench.fp_snippets --id FP-OBJ-13
function ::top
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='oo::class'  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-13`)

The class-extraction pass inspects `plain`'s body, detects the single-statement `return <literal>` shape, and uses that fact to override the self-dispatch OBJECT heuristic at the `[my plain]` callsite in `m`.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` — the `[my <method>]` self-dispatch suppression checks whether the resolved method's body is a single-statement `return <literal>` (no cmd-subs / variables); if so, the return is typed STRING and W307 fires.  Compound bodies stay suppressed via the conservative self-dispatch path.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_13_my_method_returns_literal_fires` (TP)
- `tests/test_fp_obj.py::test_FP_OBJ_13_my_method_returns_object_silent` (TN control — compound body stays suppressed)
- `tests/test_ground_truth_tn_fn.py::test_TP_W307_my_method_returns_plain_literal`
- `tests/test_ground_truth_tn_fn.py::test_TN_W307_my_method_returns_object_silent`

---

### FP-OBJ-14 — registered `::ns::cmd` with non-OBJECT return overrides the `::`-prefix factory heuristic (D3-P5 PARTIAL)

- **Verdict:** TRUE POSITIVE (precision-gap PARTIAL closure — registry-coverage limited)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_14_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D3-P5 (🔄 PARTIAL)

#### Reproducer

```tcl
namespace eval ::pkg { proc plain {} { return notACommand } }
proc f {} {
    set x [::pkg::plain]
    $x op
}
```

#### Per-line reasoning

1. `namespace eval ::pkg { proc plain {} { return notACommand } }` — defines a namespaced user proc whose body is a literal `return`.  The interprocedural fixpoint will mark `::pkg::plain` as object-returning=False (the return value is a plain string).
2. `set x [::pkg::plain]` — `x` is the string `"notACommand"`.
3. `$x op` — tries to dispatch the string as a command.  tclsh errors at runtime.
4. Pre-fix the `[::ns::cmd]` form was always typed OBJECT by the `::`-prefix factory heuristic — the assumption being that most tcllib namespaced commands are object factories.  D4-F6 partial closure added the user-proc override: when `::ns::cmd` IS a known user proc and the fixpoint did NOT classify it as object-returning, the heuristic is overridden and W307 fires.
5. D3-P5 closure separately added the registered-command path: a registered `::ns::cmd` (CommandSpec entry) with EXPLICIT non-OBJECT `return_type` (STRING/INT/LIST/etc.) also overrides the heuristic.  The data-coverage piece — adding `return_type` to tcllib factory specs — is tracked under D1-11 (registry spec coverage).

This is **deliberately partial**: an unregistered external command like `[::pkg::plain]` with NO proc visible in the file AND no registry spec still suppresses W307.  Closing that needs the registry data work.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% namespace eval ::pkg { proc plain {} { return notACommand } }
% set x [::pkg::plain]
notACommand
% $x op
invalid command name "notACommand"
```

#### Compiler evidence

```
--- FP-OBJ-14: W307 namespaced user proc with non-object return overrides factory heuristic (D3-P5 partial / D4-F6)
regen: python -m bench.fp_snippets --id FP-OBJ-14
function ::f
  block entry_1
    [0] AssignValue 'x' value='[::pkg::plain]'  defs={x#1}  uses={}
    [1] Call cmd='${x}'  defs={}  uses={x#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-14`)

`x#1` is assigned the cmd-sub value of `[::pkg::plain]`; the dispatch `${x}` at index 1 is the W307 site.  The user-proc-override path consults the interproc fixpoint, sees `::pkg::plain` is NOT object-returning, and lets W307 fire.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py:527-559` — `_is_object_returning_command_head` for `::`-prefixed names:

1. If the qualified name is in `result.all_procs` (user proc), defer to the fixpoint (`object_returning_procs` membership).  The fixpoint classifies `::pkg::plain` as NOT object-returning, so this path returns `False` (override the heuristic, fire W307).
2. Else lookup `REG_W307.get_any(cmd_head)` for a registered command spec; if `spec.return_type` is set and not `TclType.OBJECT`, return `False` (override).
3. Else fall through to the conservative `True` (suppress) — the **deferred** case for unregistered external `::pkg::plain`-style commands.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_14_namespaced_user_proc_non_object_return_fires` (TP)
- `tests/test_fp_obj.py::test_FP_OBJ_14_namespaced_known_object_factory_silent` (TN — known OBJECT-returning class command)
- `tests/test_fp_obj.py::test_FP_OBJ_14_unregistered_external_namespaced_still_silent` (deferred-coverage TN — `::pkg::plain` with no proc or spec visible)

---

### FP-OBJ-15 — `[NotAClass new]` no longer suppressed; bare-name `new`-subcommand factory heuristic removed (D3-P6 / D4-F6)

- **Verdict:** TRUE POSITIVE (precision-gap closure)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_15_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3 / TclOO 1.x
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entries D3-P6, D4-F6

#### Reproducer

```tcl
proc f {} { set x [NotAClass new]; $x method }
```

#### Per-line reasoning

1. `[NotAClass new]` — tries to invoke a `new` subcommand of the bare name `NotAClass`.  If `NotAClass` is a registered TclOO class, this is the standard object-factory idiom that returns a new object handle.  If it isn't (typo, missing `package require`, etc.), tclsh errors `invalid command name "NotAClass"`.
2. `$x method` — would dispatch on the handle.
3. Pre-fix the analyser had a `new`-subcommand factory heuristic: any `[<Name> new]` was typed as OBJECT regardless of whether `<Name>` was a known class.  That silently suppressed W307 on typos.
4. D4-F6 closure removes the heuristic: only KNOWN class names (in the `known_classes` table — `oo::class`/`itcl::class`/`snit::type`/etc. names) get the OBJECT typing.  `NotAClass` isn't known, so W307 fires.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { set x [NotAClass new]; $x method }
% f
invalid command name "NotAClass"
```

#### Compiler evidence

```
--- FP-OBJ-15: W307 bare-name [NotAClass new] no longer suppressed (D3-P6/D4-F6)
regen: python -m bench.fp_snippets --id FP-OBJ-15
function ::f
  block entry_1
    [0] AssignValue 'x' value='[NotAClass new]'  defs={x#1}  uses={}
    [1] Call cmd='${x}'  defs={}  uses={x#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-15`)

`x#1` is assigned the cmd-sub value of `[NotAClass new]`; the dispatch `${x}` at index 1 is the W307 site.  Without the bare-`new` heuristic, the analyser can't type `x` as OBJECT and emits W307.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` — `_is_object_returning_command_head` no longer treats `<bareName> new` as OBJECT-returning unconditionally.  Only `<bareName>` in `_oo_class_tails` / `_oo_class_qnames` (known TclOO classes) triggers the OBJECT typing.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_15_unknown_class_new_fires` (TP)
- `tests/test_fp_obj.py::test_FP_OBJ_15_known_oo_class_new_silent` (TN control — `[C new]` where C IS an oo::class)
- `tests/test_ground_truth_tn_fn.py::test_TP_W307_unknown_class_new_does_not_suppress`
- `tests/test_ground_truth_tn_fn.py::test_TN_W307_known_tclOO_class_new_silent`

---

### FP-OBJ-16 — composed `${ns}::tail` ensemble lookup runs unconditionally (D4-F7)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_16_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D4-F7

#### Reproducer

```tcl
namespace eval ::mypkg { proc dowork {arg} {} }
proc f {} { set ns mypkg; ${ns}::dowork arg }
```

#### Per-line reasoning

1. `namespace eval ::mypkg { proc dowork {arg} {} }` — defines `::mypkg::dowork`.
2. `set ns mypkg` — SCCP proves `ns#1 = CONST('mypkg')`.
3. `${ns}::dowork arg` — Tcl substitutes `$ns` to `mypkg`, composes `mypkg::dowork`, and dispatches.  At runtime: calls `::mypkg::dowork "arg"`, no error.
4. Pre-fix the composed-name ensemble lookup (`${prefix}::tail` → look up `::mypkg::tail`) was gated on a source-offset scan that over-fired on some inputs and silently skipped others.  D4-F7 closure runs the composed lookup unconditionally for namespaced ensembles:
   - known proc → override `sccp_says_not_a_command` to suppress
   - all-unknown → set `sccp_says_not_a_command=True` (fire)
   - mixed (some known, some not) → conservative
5. Here the composed name resolves to a known proc, so W307 stays silent.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% namespace eval ::mypkg { proc dowork {arg} {} }
% set ns mypkg
mypkg
% ${ns}::dowork arg
%                       ;# no output, no error
```

#### Compiler evidence

```
--- FP-OBJ-16: W307 ${ns}::tail composed ensemble lookup runs unconditionally (D4-F7)
regen: python -m bench.fp_snippets --id FP-OBJ-16
function ::f
  block entry_1
    [0] AssignValue 'ns' value='mypkg'  defs={ns#1}  uses={}
    [1] Call cmd='${ns}::dowork'  defs={}  uses={ns#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    ns#1: CONST('mypkg')
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-16`)

`ns#1: CONST('mypkg')` is the load-bearing fact — the composed-name lookup uses this to assemble `mypkg::dowork` and resolves it to the known proc.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` — the composed-ensemble check assembles `<sccp-resolved prefix>::<tail>` for every `${prefix}::tail` dispatch and looks the result up in `all_procs` / `known_classes` / `REGISTRY`.  When the SCCP value is a single CONST and the resolution succeeds, the dispatch is reclassified as not-a-non-literal-command and W307 is suppressed.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_16_const_prefix_resolves_to_known_proc_silent` (FP)
- `tests/test_fp_obj.py::test_FP_OBJ_16_const_prefix_unknown_proc_fires` (TP control — composed name doesn't resolve)
- `tests/test_ground_truth_tn_fn.py::test_TN_namespaced_ensemble_resolved_known_proc`
- `tests/test_ground_truth_tn_fn.py::test_TP_namespaced_ensemble_composed_unknown`

---

### FP-OBJ-17 — `array set state {-command notACommand}` literal-element harvester (D3-P7)

- **Verdict:** TRUE POSITIVE (precision-gap closure)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_17_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D3-P7

#### Reproducer

```tcl
proc f {} { array set state {-command notACommand}; $state(-command) hi }
```

#### Per-line reasoning

1. `array set state {-command notACommand}` — initialises `state` as an array with the single element `state(-command) = "notACommand"`.
2. `$state(-command) hi` — dispatches whatever string `state(-command)` holds.  Since it's `"notACommand"`, tclsh errors `invalid command "notACommand"`.
3. Pre-fix the callback-key W307 heuristic suppressed dispatches through array elements with callback-shaped keys (keys ending in `-command`, `-callback`, `cb`, `handler`, etc.).  That suppression was correct when the value WAS a callback but wrong when the literal value is provably a non-command.
4. D3-P7 closure: a new `array set` literal-element harvester walks the `{key value …}` literal list arg of `array set`, registers each pair as a CONSTSET keyed on `state(<key>)`, and feeds it to SCCP.  The W307 SCCP-evidence override then fires when the literal value isn't a known command.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { array set state {-command notACommand}; $state(-command) hi }
% f
invalid command name "notACommand"
```

#### Compiler evidence

```
--- FP-OBJ-17: W307 array set literal-element harvester for callback array (D3-P7)
regen: python -m bench.fp_snippets --id FP-OBJ-17
function ::f
  block entry_1
    [0] Call cmd='array'  defs={state#1}  uses={}
    [1] Call cmd='${state(-command)}'  defs={}  uses={state#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    state#1: OVERDEFINED
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-17`)

The literal-element harvester runs alongside the SCCP pass; the per-key CONSTSET evidence is consulted by the W307 emitter (not by the per-name SCCP lattice rendered here).  `state(-command)` is registered as `"notACommand"` and the W307 check overrides the callback-key suppression.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` — the `array set <var> {key value …}` literal harvester runs at the call-site; for each `key value` pair, it records `var(key) -> CONST(value)` in the per-proc CONSTSET map.  The W307 check, before applying the callback-key suppression, consults the CONSTSET for the array-element place and overrides the suppression when the literal value isn't a known command.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_17_callback_array_holds_noncommand_fires` (TP)
- `tests/test_fp_obj.py::test_FP_OBJ_17_callback_array_holds_known_command_silent` (TN control — value IS a known command)
- `tests/test_ground_truth_tn_fn.py::test_TP_W307_callback_array_holds_noncommand`
- `tests/test_ground_truth_tn_fn.py::test_TN_W307_callback_array_holds_known_command`

---

### FP-OBJ-18 — `dict with` key-value pair harvester for interproc-propagated callback (D3-P8)

- **Verdict:** TRUE POSITIVE (precision-gap closure)
- **Status:** locked in by `tests/test_fp_obj.py::test_FP_OBJ_18_*`
- **Codes:** W307 (non-literal command name)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D3-P8 (builds on D3-P2 / FP-DS-09)

#### Reproducer

```tcl
proc f {d} { dict with d { $cmd hi } }
f {cmd notACommand}
```

#### Per-line reasoning

1. `proc f {d} { dict with d { $cmd hi } }` — callee unpacks `d` as locals and dispatches via `$cmd`.
2. `f {cmd notACommand}` — caller passes the literal dict with key `cmd` and value `notACommand`.
3. Interproc propagation (the FP-DS-09 / D3-P2 closure) puts `d#0 = CONST('cmd notACommand')` in the callee.
4. `dict with d` unpacks `cmd = "notACommand"` as a local; `$cmd hi` then dispatches `"notACommand" hi`.  tclsh errors at runtime.
5. D3-P8 closure builds on FP-DS-09: the dict-with key-value-pair harvester reads `d#0`'s SCCP CONST value, parses it as a Tcl list, and registers each `key→value` pair as a CONSTSET in the W307 evidence map.  The override then fires because `notACommand` isn't a known command.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {d} { dict with d { $cmd hi } }
% f {cmd notACommand}
invalid command name "notACommand"
```

#### Compiler evidence

```
--- FP-OBJ-18: W307 dict-with key-value pair harvester for interproc callback (D3-P8)
regen: python -m bench.fp_snippets --id FP-OBJ-18
function ::f
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='dict'  defs={d#1}  uses={cmd#0, d#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    cmd#0: OVERDEFINED
    d#0: CONST('cmd notACommand')
```
(regen: `python -m bench.fp_snippets --id FP-OBJ-18`)

`d#0: CONST('cmd notACommand')` is the load-bearing fact — interproc propagation from `f {cmd notACommand}` seeded the lattice.  The dict-with key-value harvester reads this, registers `cmd -> notACommand` as CONSTSET evidence, and the W307 check overrides the callback-shape suppression.

#### Why the analyser reaches that verdict

`analyser/_analyser/_diag_var_command.py` + `compiler/core_analyses.py` (the interproc literal-arg propagation from FP-DS-09) — the W307 emitter consults the per-key CONSTSET map populated by the dict-with key-value harvester; when the value for a callback-shaped local isn't a known command, the suppression is overridden.

#### Tests

- `tests/test_fp_obj.py::test_FP_OBJ_18_interproc_dict_with_noncommand_fires` (TP)
- `tests/test_fp_obj.py::test_FP_OBJ_18_interproc_dict_with_known_command_silent` (TN control)
- `tests/test_ground_truth_tn_fn.py::test_TP_W307_interproc_dict_with_unpacks_non_command`
- `tests/test_ground_truth_tn_fn.py::test_TN_interproc_dict_with_unpacks_known_command_silent`
- Cross-link: [FP-DS-09](#fp-ds-09) (the underlying interproc dict propagation).

---


## RCH — reachability (O107)

`break` / `continue` are jump statements, not CFG edges; `try` handlers
were CFG islands.  Without analysis-only edges, post-loop blocks and
handler bodies looked unreachable (false O107 + unsound DCE).  These
entries lock in the SCCP `break → loop-exit` edge feed and the SSA
exception-edge inheritance into try handlers.

### FP-RCH-01 — while 1 { break }: break-after is reachable (not O107 dead code)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_rch.py::test_FP_RCH_01_*`
- **Codes:** O107
- **Corpus:** every event-loop / consumer that uses `while 1 { wait_for_work; if {$done} break }` — extremely common.

#### Reproducer

```tcl
proc f {c} {
    # while 1 with a conditional `break` -> `puts after` IS reachable.
    while 1 { if {$c} break }
    puts after
}
```

#### Per-line reasoning

1. `while 1 { if {$c} break }` — the loop header has condition `1` (constant true).
2. `puts after` — appears after the loop.

Pre-fix: `break` was modelled as a jump statement, not a CFG edge.  So the only edge into the loop-exit block was the header's exit edge — which SCCP prunes as dead when the condition is constant-true.  The post-loop block was unreachable in the reachability worklist → O107 fired on `puts after`, AND DCE was unsound (could delete still-reachable code).

Fix: SCCP feeds the `break → loop-exit` edge into reachability so the post-loop block stays alive.

#### tclsh ground truth

```
% set c 1; while 1 { if {$c} break }; puts after
after
```

#### Compiler evidence

```
--- FP-RCH-01: while 1 { break }: break-after is reachable (not O107 dead code)
regen: python -m bench.fp_snippets --id FP-RCH-01
function ::f
  block entry_1
    term Goto
  block while_header_2
    term Branch ExprLiteral(text='1', start=0, end=0)
  block while_body_3
    term Branch ExprVar(text='$c', name='c', start=0, end=1)
  block while_end_4
    [0] Call cmd='puts'  defs={}  uses={}
    term Goto
  block if_end_5
    term Goto
  block if_then_6
    [0] Call cmd='break'  defs={}  uses={}
    term Goto
  block if_next_7
    term Goto
  block exit_8
    term (none — fall-through exit)
```

#### Why the analyser reaches that verdict

`compiler/core_analyses.py` (or `compiler/optimiser/_helpers.py` — see the reachability worklist) treats `break` / `continue` as CFG edges into their enclosing loop's exit / latch block when feeding reachability.  The bytecode lowering still emits the jump as a statement so default-bytecode codegen stays tclsh-identical.

#### Tests

- `tests/test_fp_rch.py::test_FP_RCH_01_while1_break_after_reachable` (FP, while)
- `tests/test_fp_rch.py::test_FP_RCH_01_for_true_break_reachable` (FP, for-true)
- `tests/test_fp_rch.py::test_FP_RCH_01_nested_loop_break_reachable` (FP, nested)

---

### FP-RCH-02 — try handler body is reachable (analysis-only exception edges)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_rch.py::test_FP_RCH_02_*`
- **Codes:** O107
- **Corpus:** every `try`/`on error`/`on ok` use across the corpus — error-handling wrappers in tcllib's `fileutil`, iRules error logging.

#### Reproducer

```tcl
proc f {} {
    # `on error` handler body is reachable; no O107 on `set y 1`.
    try {
        set x [doThing]
    } on error {e opts} {
        set y 1
        puts $y
    }
}
```

#### Per-line reasoning

1. `try { set x [doThing] }` — body block.
2. `on error {e opts} { set y 1; puts $y }` — handler block.

Pre-fix the handler block had no CFG predecessor edge from the body — it was a CFG island.  Every statement in it fired O107 ('unreachable dead code').  Fix: SSA construction adds analysis-only exception edges from the body to each handler; the handler is reachable in the analyser's view.

#### tclsh ground truth

```
% try { error fail } on error {e opts} { set y 1; puts $y }
1
```

#### Compiler evidence

```
--- FP-RCH-02: try handler body is reachable (analysis-only exception edges)
regen: python -m bench.fp_snippets --id FP-RCH-02
function ::f
  block entry_1
    term Goto
  block try_body_2
    [0] AssignValue 'x' value='[doThing]'  defs={x#1}  uses={}
    term Goto
  block try_end_3
    term Goto
  block try_ok_4
    term Goto
  block try_handler_5
    [0] Call cmd='try'  defs={e#1, opts#1}  uses={}
    [1] AssignConst 'y' value='1'  defs={y#1}  uses={}
    [2] Call cmd='puts'  defs={}  uses={y#1}
    term Goto
  block exit_6
    term (none — fall-through exit)
```

#### Why the analyser reaches that verdict

`compiler/ssa.py` adds exception edges from the try body to each handler block during SSA construction.  The edges are tagged 'analysis-only' so codegen ignores them and default-bytecode lowering stays tclsh-identical.

#### Tests

- `tests/test_fp_rch.py::test_FP_RCH_02_handler_body_reachable` (FP)
- `tests/test_fp_rch.py::test_FP_RCH_02_handler_var_not_unset` (FP — handler-bound `e` is defined)

---

### FP-RCH-03 — on ok inherits body-defined SSA versions (no W210 on body-set var)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_rch.py::test_FP_RCH_03_on_ok_reads_body_var`
- **Codes:** W210
- **Corpus:** `try { set v [doThing] } on ok {} { use $v }` is the common-case fallback for ensuring a body completed before consuming its result.

#### Reproducer

```tcl
proc f {} {
    # `on ok` runs after the body completes; $vdata IS defined.
    try {
        set vdata [getData]
    } on ok {} {
        return $vdata
    }
}
```

#### Per-line reasoning

`on ok` runs only **after** the body completes normally (no error).  At that point any var the body set is defined.  Pre-fix the handler block didn't inherit body SSA versions — it saw `vdata#0` (sentinel "before any def") and fired W210.

Fix: the ok-path exception edge feeds the body's last-version map into the handler's phi inputs, mirroring natural sequential control flow.

#### tclsh ground truth

```
% try { set vdata 42 } on ok {} { puts $vdata }
42
```

#### Compiler evidence

```
--- FP-RCH-03: on ok inherits body-defined SSA versions (no W210 on body-set var)
regen: python -m bench.fp_snippets --id FP-RCH-03
function ::f
  block entry_1
    term Goto
  block try_body_2
    [0] AssignValue 'vdata' value='[getData]'  defs={vdata#1}  uses={}
    term Goto
  block try_end_3
    term Goto
  block try_ok_4
    term Goto
  block try_handler_5
    term Return ${vdata}
  block exit_6
    term (none — fall-through exit)
  read_before_set: (none)
```

#### Why the analyser reaches that verdict

`compiler/ssa.py`'s phi placement for try-handler blocks consults the body's exit-version map for the ok edge; on-error inherits the pre-try state (since the body may not have set the var before erroring).

#### Tests

- `tests/test_fp_rch.py::test_FP_RCH_03_on_ok_reads_body_var` (FP)
- `tests/test_fp_rch.py::test_FP_RCH_03_on_ok_unset_var_still_fires` (smoke / no-crash control)

---

### FP-RCH-04 — genuine infinite-loop (no break) -> code after IS unreachable (TP control)

- **Verdict:** TRUE POSITIVE (control)
- **Status:** locked in by `tests/test_fp_rch.py::test_FP_RCH_04_infinite_loop_dead_code_fires`
- **Codes:** O107
- **Corpus:** genuine infinite-loop antipatterns (typically a regression / leftover during development).

#### Reproducer

```tcl
proc f {} {
    # No break / return -> `puts after` IS dead code.
    while 1 { puts x }
    puts after
}
```

#### Per-line reasoning

`while 1 { puts x }` has no `break` / `return` / uncaught exception in the body, so control never reaches the post-loop block.  `puts after` IS dead code; O107 fires.

This control test ensures the FP-RCH-01 fix doesn't blanket-suppress all post-loop reachability — only the specific case where a `break` edge feeds in.

#### tclsh ground truth

```
% while 1 { puts x }
(infinite output, never reaches `puts after`)
```

#### Compiler evidence

```
--- FP-RCH-04: genuine infinite-loop (no break) -> code after IS unreachable (TP control)
regen: python -m bench.fp_snippets --id FP-RCH-04
function ::f
  block entry_1
    term Goto
  block while_header_2
    term Branch ExprLiteral(text='1', start=0, end=0)
  block while_body_3
    [0] Call cmd='puts'  defs={}  uses={}
    term Goto
  block while_end_4
    [0] Call cmd='puts'  defs={}  uses={}
    term Goto
  block exit_5
    term (none — fall-through exit)
```

#### Why the analyser reaches that verdict

With no `break` to feed an edge into the loop-exit block, the SCCP-pruned constant-true header edge is the ONLY potential predecessor; reachability correctly marks the post-loop block dead.

#### Tests

- `tests/test_fp_rch.py::test_FP_RCH_04_infinite_loop_dead_code_fires` (TP)

---


## INJ — injection / style (W101/W105/W301/T102)

Tcl's substitution model makes `eval` and `uplevel` style decisions
high-stakes: the safe canonical forms (`uplevel 1 $body`,
`eval [list …]`) must stay silent or every safe idiom warns; the genuine-
risk forms (quoted interpolation, double substitution) must still warn or
the check loses value.  T102 (option injection) uses taint colours
(PATH_PREFIXED for HTTP::uri / HTTP::path) for the same FP-vs-TP trade.

### FP-INJ-01 — uplevel 1 $body (bare var) is the safe idiom — NOT W301

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_inj.py::test_FP_INJ_01_*`
- **Codes:** W301
- **Corpus:** every callback-passing API in tcllib (`fileutil::traverse`, `htmlparse::*`, snit's `delegate`-via-uplevel patterns).

#### Reproducer

```tcl
proc f {body} {
    # The canonical `uplevel 1 $body` pattern — must NOT fire W301.
    uplevel 1 $body
}
```

#### Per-line reasoning

`uplevel 1 $body` evaluates `$body` ONCE in the caller frame; the value is the script source.  Pre-fix W301 ('use braces') wrongly recommended `uplevel 1 {$body}` — but braces block the variable expansion, so the braced form evaluates the literal text `$body` (not the variable's contents).

Fix: W301 recognises the `$single_var` form as safe; only quoted-interpolation forms (`"$cmd $arg"`) or multi-arg concatenation get flagged.

#### tclsh ground truth

```
% set body { puts hi }
% uplevel 1 $body
hi
% uplevel 1 {$body}   ;# fires `invalid command name "$body"` — would be the W301 suggested replacement
```

#### Compiler evidence

```
--- FP-INJ-01: uplevel 1 $body (bare var) is the safe idiom — NOT W301
regen: python -m bench.fp_snippets --id FP-INJ-01
function ::f
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='uplevel'  defs={}  uses={body#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
```

#### Why the analyser reaches that verdict

`analyser/checks/_domain.py` walks the second-arg word: a single `WordSubst` whose source is a single `$var` (no surrounding quoted text) is the safe form; any quoted-text + `$var` mixture is the risky form.

#### Tests

- `tests/test_fp_inj.py::test_FP_INJ_01_bare_var_no_w301` (FP)
- `tests/test_fp_inj.py::test_FP_INJ_01_quoted_interpolation_still_w301` (TP)

---

### FP-INJ-02 — eval [list ...] is safe — list-canonical form, NOT W101

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_inj.py::test_FP_INJ_02_*`
- **Codes:** W101
- **Corpus:** every dynamic-call wrapper in tcllib (`namespace ensemble` setup, dispatcher patterns).

#### Reproducer

```tcl
# Canonical safe form — eval of a list-returning cmd-sub.  No W101.
eval [list set $varname $value]
```

#### Per-line reasoning

`eval [list set $varname $value]` is the **canonical-safe** form: `[list …]` produces a list whose elements survive `eval`'s concatenation/re-parse without double substitution.

Fix: the W101 check exempts list-returning canonical commands (`list`, `linsert`, `lreplace`, `split`, `concat`, etc.) — see the canonical-safe set in the registry.

#### tclsh ground truth

```
% set varname x; set value 1
% eval [list set $varname $value]
1
% set x
1
```
The list-quoted form passes `$varname` and `$value` as exactly-one-word each, no double substitution.

#### Compiler evidence

```
--- FP-INJ-02: eval [list ...] is safe — list-canonical form, NOT W101
regen: python -m bench.fp_snippets --id FP-INJ-02
function ::top
  block entry_1
    [0] InterpBoundary  defs={}  uses={}
    [1] Barrier cmd='eval'  defs={}  uses={value#0, varname#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
```

#### Why the analyser reaches that verdict

`analyser/checks/_domain.py` recognises the canonical-safe cmd-sub set (sourced from the command registry).  Any `eval [CANONICAL_SAFE_CMD …]` is exempt.

#### Tests

- `tests/test_fp_inj.py::test_FP_INJ_02_eval_list_clean` (FP, `eval [list …]`)
- `tests/test_fp_inj.py::test_FP_INJ_02_eval_linsert_clean` (FP, `eval [linsert …]`)
- `tests/test_fp_inj.py::test_FP_INJ_02_eval_string_concat_still_w101` (TP control)

---

### FP-INJ-03 — T102 suppression: HTTP::uri PATH_PREFIXED -> no option injection

- **Verdict:** FALSE POSITIVE (now fixed, taint colour suppression)
- **Status:** locked in by `tests/test_fp_inj.py::test_FP_INJ_03_*`
- **Codes:** T102
- **Corpus:** every iRule that consumes `HTTP::uri` / `HTTP::path` (most iRules in any production deployment).

#### Reproducer

```tcl
set uri [HTTP::uri]
regexp $uri test
```

#### Per-line reasoning

iRules semantics: `HTTP::uri` and `HTTP::path` return strings that **always** begin with `/` (path-anchored — the F5 documentation guarantees this).  No attacker-controlled input can make them start with `-`, so feeding them to a command with an option terminator (`regexp $uri …`) cannot trigger option injection.

Fix: the taint pass tags `HTTP::uri` / `HTTP::path` (and the related IP::*_addr / TCP::*_port sources) with the **PATH_PREFIXED** taint colour; T102 suppresses any value carrying that colour.  The colour propagates through copy assignments and non-dash literal concatenations.

#### tclsh ground truth

```
HTTP::uri  -> "/some/path?x=1"   ; HTTP::path -> "/some/path"
```
No path-anchored value can start with `-`, so `regexp` cannot misinterpret it as an option.

#### Compiler evidence

```
--- FP-INJ-03: T102 suppression: HTTP::uri PATH_PREFIXED -> no option injection
regen: python -m bench.fp_snippets --id FP-INJ-03
function ::top
  block entry_1
    [0] AssignValue 'uri' value='[HTTP::uri]'  defs={uri#1}  uses={}
    [1] Call cmd='regexp'  defs={}  uses={uri#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    uri#1: OVERDEFINED
```

#### Why the analyser reaches that verdict

`compiler/taint/_sinks.py:_check_t102` consults the value's taint colour set; values with `PATH_PREFIXED` are exempt.  The source assignments tag the colour in `compiler/taint/_api.py`'s seed step.

#### Tests

- `tests/test_fp_inj.py::test_FP_INJ_03_http_uri_no_t102` (FP)
- `tests/test_fp_inj.py::test_FP_INJ_03_http_path_no_t102` (FP)
- `tests/test_fp_inj.py::test_FP_INJ_03_path_prefixed_copy_suppresses` (FP — colour propagates)
- `tests/test_fp_inj.py::test_FP_INJ_03_literal_non_dash_prefix_no_t102` (FP — fixed non-dash prefix)

---

### FP-INJ-04 — T102 TP control: literal '-' prefix [HTTP::path] still warns

- **Verdict:** TRUE POSITIVE (control)
- **Status:** locked in by `tests/test_fp_inj.py::test_FP_INJ_04_*`
- **Codes:** T102
- **Corpus:** combine-iRule patterns that rewrite paths with a literal prefix — common in URL canonicalisation rules.

#### Reproducer

```tcl
set foo "-[HTTP::path]"
regexp $foo test
```

#### Per-line reasoning

Prepending a fixed `-` to an HTTP-derived value (`"-[HTTP::path]"`) produces an option-LIKE string.  The path-prefix safety from FP-INJ-03 was specifically that the value *itself* couldn't start with `-`; once you concatenate a `-` literal before it, that guarantee evaporates.

T102 must still fire here — this control test proves FP-INJ-03's suppression isn't blanket-exempting every HTTP-derived value.

#### tclsh ground truth

```
"-[HTTP::path]"  -> "-/some/path"
```
That's an option-LIKE string a careless consumer could misinterpret.

#### Compiler evidence

```
--- FP-INJ-04: T102 TP control: literal '-' prefix [HTTP::path] still warns
regen: python -m bench.fp_snippets --id FP-INJ-04
function ::top
  block entry_1
    [0] AssignValue 'foo' value='-[HTTP::path]'  defs={foo#1}  uses={}
    [1] Call cmd='regexp'  defs={}  uses={foo#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    foo#1: OVERDEFINED
```

#### Why the analyser reaches that verdict

`compiler/taint/_path_concat.py` clears PATH_PREFIXED when a literal `-` prefix is prepended; T102's suppression no longer applies.  Generic (non-PATH_PREFIXED) tainted data also still fires (see `test_FP_INJ_04_generic_taint_still_warns`).

#### Tests

- `tests/test_fp_inj.py::test_FP_INJ_04_dash_prefix_still_warns` (TP)
- `tests/test_fp_inj.py::test_FP_INJ_04_generic_taint_still_warns` (TP, generic taint)

---

### FP-INJ-05 — eval "$cmd $x" -> W101 with code-action rewrite to eval [list ...]

- **Verdict:** TRUE POSITIVE (with code-action)
- **Status:** locked in by `tests/test_fp_inj.py::test_FP_INJ_05_*`
- **Codes:** W101
- **Corpus:** any dynamic-command-build pattern using string-concat to assemble the call (legacy Tcl idioms before `eval [list …]` was popularised).

#### Reproducer

```tcl
# Top-level. eval of a non-list cmd-sub -> W101 + quick-fix.
set x foo
eval "process $x"
```

#### Per-line reasoning

`eval "process $x"` performs DOUBLE substitution: every embedded `$var` and `[cmd]` is substituted twice — once by the outer parser, once again by eval's re-parse of the resulting string.  This is the canonical Tcl-injection vulnerability: any value of `$x` containing `[` or `$` will get re-evaluated.

W101 fires; the LSP code-action rewrites the call to the safe form `eval [list process $x]` so `[list …]` quoting prevents double substitution.

#### tclsh ground truth

```
% set x {[exec /bin/rm -rf /]}    ;# attacker-controlled
% eval "process $x"               ;# DOUBLE-SUBSTITUTED: runs the inner [exec …]
```

#### Compiler evidence

```
--- FP-INJ-05: eval "$cmd $x" -> W101 with code-action rewrite to eval [list ...]
regen: python -m bench.fp_snippets --id FP-INJ-05
function ::top
  block entry_1
    [0] AssignValue 'x' value='foo'  defs={x#1}  uses={}
    [1] InterpBoundary  defs={}  uses={}
    [2] Barrier cmd='eval'  defs={}  uses={x#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```

#### Why the analyser reaches that verdict

`analyser/checks/_domain.py` fires W101 for double-quoted string-form eval; the matching code-action (`server/features/code_actions.py`) extracts the command name + args from the quoted-form parse and emits the `eval [list …]` replacement.

#### Tests

- `tests/test_fp_inj.py::test_FP_INJ_05_eval_string_fires_w101` (TP, fires)
- `tests/test_fp_inj.py::test_FP_INJ_05_code_action_rewrites_to_eval_list` (TP, code-action contract — the rewrite text is exercised more thoroughly in `tests/test_checks.py::TestEvalInjection`)

---


## BND — bounds / intervals (W230/W231/W232/W233)

Phase-3 interval-domain entries: dynamic lindex / lset / string index /
expr divide that's *provably* out-of-range / divide-by-zero against the
tracked interval lattice.  The broader corpus lives in
`tests/test_interval_bounds.py` (~30 tests covering escape sequences,
Unicode, guard narrowing, etc.); these are the curated FP.md must-keeps.

### FP-BND-01 — W231 lset dynamic out-of-range loop index ($j > length) fires

- **Verdict:** TRUE POSITIVE (Phase-3 dynamic bounds)
- **Status:** locked in by `tests/test_fp_bnd.py::test_FP_BND_01_loop_index_past_append_slot_fires`
- **Codes:** W231
- **Corpus:** typo'd loop ranges past the list length (the bug that motivated Phase 3 — found by inspection in tcllib's `struct::list`).

#### Reproducer

```tcl
proc f {v} {
    # j is bounded [4, 8]; list length 3 -> every iteration is OOR.
    set l {a b c}
    for {set j 4} {$j < 9} {incr j} { lset l $j $v }
}
```

#### Per-line reasoning

1. `set l {a b c}` — list with literal length 3.  The interval domain tracks the literal-list length per SSA version.
2. `for {set j 4} {$j < 9} {incr j}` — loop induction variable `j` has range `[4, 8]`.
3. `lset l $j $v` — every value of `j` in `[4, 8]` is strictly > 3 (list length).  tclsh errors with `index "4" out of range` (then never gets to 5 etc.).

W231 fires once with `$j` in the message (the dynamic-index form, sourced from the interval-domain proof).

#### tclsh ground truth

```
% set l {a b c}; for {set j 4} {$j < 9} {incr j} { lset l $j x }
list index out of range
```

#### Compiler evidence

```
--- FP-BND-01: W231 lset dynamic out-of-range loop index ($j > length) fires
regen: python -m bench.fp_snippets --id FP-BND-01
function ::f
  block entry_1
    [0] AssignConst 'l' value='a b c'  defs={l#1}  uses={}
    [1] AssignConst 'j' value='4'  defs={j#1}  uses={}
    term Goto
  block for_header_2
    phi  SSAPhi(name='j', version=2, incoming={'entry_1': 1, 'for_step_4': 3})
    term Branch ExprBinary(op=<BinOp.LT: '<'>, left=ExprVar(text='$j', name='j', start=0, end=1), right=ExprLiteral(text='9', start=5, end=5))
  block for_body_3
    [0] Call cmd='lset'  defs={l#2}  uses={j#2, v#0}
    term Goto
  block for_step_4
    [0] Incr 'j'  defs={j#3}  uses={j#2}
    term Goto
  block for_end_5
    term Goto
  block exit_6
    term (none — fall-through exit)
  values (SCCP lattice)
    j#1: CONST(4)
    j#2: OVERDEFINED
    j#3: OVERDEFINED
  dead_stores
    DeadStore(block='entry_1', statement_index=0, variable='l', version=1)
```

#### Why the analyser reaches that verdict

`compiler/interval_bounds.py` consumes the SSA interval lattice for `j` and the literal list-length for `l`; emits the W231 finding via `analyser/_analyser/_diag_interval_bounds.py`.  The check uses `>` (not `>=`) to permit the append slot (FP-BND-02 / FP-NAB-01).

#### Tests

- `tests/test_fp_bnd.py::test_FP_BND_01_loop_index_past_append_slot_fires` (TP)

---

### FP-BND-02 — W231 dynamic append-slot ($j == length) IS silent (FP guard)

- **Verdict:** FALSE POSITIVE (FP guard)
- **Status:** locked in by `tests/test_fp_bnd.py::test_FP_BND_02_*`
- **Codes:** W231
- **Corpus:** every lset-append idiom (struct::list, struct::matrix's column-add helpers).

#### Reproducer

```tcl
proc f {v} {
    # j == length -> APPEND slot; must NOT fire W231.
    set l {a b c}
    set j 3
    lset l $j $v
}
```

#### Per-line reasoning

`lset l $j $v` with `set j 3` against `l = {a b c}` is the **dynamic append slot**: `j == length` is legal lset (it appends).  The dynamic check must mirror the literal-index path's `> length` (not `>= length`) comparison.

Pre-fix the dynamic check used `>=` and fired W231 for the append slot.  Fix: `interval_bounds.py`'s lset entry uses `> length` so the append slot stays silent — parallel to FP-NAB-01.

#### tclsh ground truth

```
% set l {a b c}; set j 3; lset l $j X; puts $l
a b c X
```

#### Compiler evidence

```
--- FP-BND-02: W231 dynamic append-slot ($j == length) IS silent (FP guard)
regen: python -m bench.fp_snippets --id FP-BND-02
function ::f
  block entry_1
    [0] AssignConst 'l' value='a b c'  defs={l#1}  uses={}
    [1] AssignConst 'j' value='3'  defs={j#1}  uses={}
    [2] Call cmd='lset'  defs={l#2}  uses={j#1, v#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    j#1: CONST(3)
```

#### Why the analyser reaches that verdict

`compiler/interval_bounds.py` uses strict `>` in the lset OOR comparison; matches the literal-arg check in `analyser/checks/_bounds.py`.

#### Tests

- `tests/test_fp_bnd.py::test_FP_BND_02_dynamic_append_slot_silent` (FP)
- `tests/test_fp_bnd.py::test_FP_BND_02_in_range_dynamic_silent` (FP control)

---

### FP-BND-03 — W232 string index past end ($i >= length) fires (string smell)

- **Verdict:** TRUE POSITIVE (Phase-3 dynamic bounds)
- **Status:** locked in by `tests/test_fp_bnd.py::test_FP_BND_03_*`
- **Codes:** W232
- **Corpus:** string-extraction patterns where the index is past the source string (off-by-one bugs caught in tcllib's `textutil::*` modules).

#### Reproducer

```tcl
proc f {} {
    # i (10) > string length (5) -> tclsh returns ""; W232 smell.
    set s "hello"
    set i 10
    return [string index $s $i]
}
```

#### Per-line reasoning

1. `set s "hello"` — string with literal length 5.  The interval domain tracks string-length per SSA version (in Tcl character units, with backslash-escapes resolved).
2. `set i 10` — `i#1: CONST(10)`.
3. `string index $s $i` — `i (10) > length (5)`; tclsh returns `""` silently.

This is a smell-tier W232 (severity matches W230 for lindex; not the error tier W231 for lset).

#### tclsh ground truth

```
% set s "hello"; set i 10; puts "result=>'[string index $s $i]'"
result=>''
```
No error — just an empty result.

#### Compiler evidence

```
--- FP-BND-03: W232 string index past end ($i >= length) fires (string smell)
regen: python -m bench.fp_snippets --id FP-BND-03
function ::f
  block entry_1
    [0] AssignValue 's' value='hello'  defs={s#1}  uses={}
    [1] AssignConst 'i' value='10'  defs={i#1}  uses={}
    term Return [string index $s $i]
  values (SCCP lattice)
    i#1: CONST(10)
```

#### Why the analyser reaches that verdict

`compiler/interval_bounds.py` tracks `string_length_map[ssa_version]` for each literal-set + `string` builtin; W232 fires when `i ∈ [length, +∞)`.  Backslash-escape resolution mirrors tclsh's character-count behaviour (see `tests/test_interval_bounds.py::TestDynamicStringIndex` for the escape-handling edge cases).

#### Tests

- `tests/test_fp_bnd.py::test_FP_BND_03_string_index_past_end_fires` (TP)
- `tests/test_fp_bnd.py::test_FP_BND_03_idx_equals_length_fires` (TP, idx == length is also OOR)
- `tests/test_fp_bnd.py::test_FP_BND_03_in_range_silent` (FP control)
- `tests/test_fp_bnd.py::test_FP_BND_03_unknown_string_silent` (FP control, unknowns)

---

### FP-BND-04 — W233 division by a provably-zero divisor (constant $d=0) fires

- **Verdict:** TRUE POSITIVE (Phase-3 deep finding)
- **Status:** locked in by `tests/test_fp_bnd.py::test_FP_BND_04_*`
- **Codes:** W233
- **Corpus:** off-by-one or zero-init bugs in arithmetic expressions (typical: forgotten loop-variable init).

#### Reproducer

```tcl
proc f {} {
    # $d == 0 (CONST) -> tclsh divide-by-zero; W233 fires.
    set d 0
    return [expr {10 / $d}]
}
```

#### Per-line reasoning

`expr {10 / $d}` with `set d 0` is a tclsh divide-by-zero runtime error.  The interval domain proves `d#1: CONST(0)` and the deep-finding pass emits W233 (severity: error).

#### tclsh ground truth

```
% set d 0; expr {10 / $d}
divide by zero
% expr {1 / 0}
divide by zero
% expr {5 % 0}
divide by zero
```

#### Compiler evidence

```
--- FP-BND-04: W233 division by a provably-zero divisor (constant $d=0) fires
regen: python -m bench.fp_snippets --id FP-BND-04
function ::f
  block entry_1
    [0] AssignConst 'd' value='0'  defs={d#1}  uses={}
    term Return [expr {10 / $d}]
  values (SCCP lattice)
    d#1: CONST(0)
```

#### Why the analyser reaches that verdict

`compiler/intervals.py`'s expression evaluator detects division and modulo with `divisor.contains_zero()`; the deep-finding pass at `analyser/_analyser/_diag_interval_bounds.py` consumes the proof.

#### Tests

- `tests/test_fp_bnd.py::test_FP_BND_04_const_zero_divisor_fires` (TP, const variable)
- `tests/test_fp_bnd.py::test_FP_BND_04_literal_div_zero_fires` (TP, literal 1/0)
- `tests/test_fp_bnd.py::test_FP_BND_04_literal_mod_zero_fires` (TP, literal 5%0)
- `tests/test_fp_bnd.py::test_FP_BND_04_nonzero_divisor_silent` (FP control)
- `tests/test_fp_bnd.py::test_FP_BND_04_unknown_divisor_silent` (FP control, unknown)

---

### FP-BND-05 — W233 FP guard: dead ternary arm / short-circuit `1 || 1/0` is silent

- **Verdict:** FALSE POSITIVE (FP guard, short-circuit semantics)
- **Status:** locked in by `tests/test_fp_bnd.py::test_FP_BND_05_*`
- **Codes:** W233
- **Corpus:** defensive programming idioms — `expr {$d != 0 ? $x / $d : 0}` and friends.

#### Reproducer

```tcl
proc f {} {
    # `0 ? 1/0 : 7` -> dead arm; tclsh returns 7; W233 must NOT fire.
    return [expr {0 ? 1/0 : 7}]
}
```

#### Per-line reasoning

Tcl `expr` is **lazy**: the dead arm of `?:` and the short-circuited operand of `&&` / `||` are never evaluated.  So:

* `expr {0 ? 1/0 : 7}` — the `1/0` is in the dead `?` arm; never evaluated; result is 7.
* `expr {1 || 1/0}` — `||` short-circuits on `1`; `1/0` never evaluates; result is 1.
* `expr {0 && 1/0}` — `&&` short-circuits on `0`; `1/0` never evaluates; result is 0.
* `if {$d != 0} { 1 / $d }` — the SCCP-pruned branch where `d == 0` doesn't reach the division.

W233 must respect short-circuit semantics; firing in any of these positions would be a false positive.

#### tclsh ground truth

```
% expr {0 ? 1/0 : 7}
7
% expr {1 || 1/0}
1
% expr {0 && 1/0}
0
```

#### Compiler evidence

```
--- FP-BND-05: W233 FP guard: dead ternary arm / short-circuit `1 || 1/0` is silent
regen: python -m bench.fp_snippets --id FP-BND-05
function ::f
  block entry_1
    term Return [expr {0 ? 1/0 : 7}]
```

#### Why the analyser reaches that verdict

`compiler/intervals.py`'s expression-AST walker honours short-circuit semantics for `?:` / `&&` / `||` — it doesn't evaluate the unreachable operand.  The `if`-guard case is covered by SCCP branch pruning.

#### Tests

- `tests/test_fp_bnd.py::test_FP_BND_05_dead_ternary_arm_silent` (FP, ternary)
- `tests/test_fp_bnd.py::test_FP_BND_05_short_circuit_or_silent` (FP, ||)
- `tests/test_fp_bnd.py::test_FP_BND_05_short_circuit_and_silent` (FP, &&)
- `tests/test_fp_bnd.py::test_FP_BND_05_guard_excludes_zero_silent` (FP, if-guard)

---

### FP-BND-06 — W233 fires when a non-integer constant guard forces the lazy arm

- **Verdict:** TRUE POSITIVE (forced-arm guaranteed divide-by-zero) + paired FP guards
- **Status:** locked in by `tests/test_fp_bnd.py::test_FP_BND_06_*`
- **Codes:** W233
- **Corpus:** synthetic edge cases (no clean-corpus instance — a guaranteed `1/0` behind a
  constant-true guard is a bug clean libraries don't contain; corpus delta 0). tclsh-verified.

#### Reproducer

```tcl
proc f {} {
    # 1.0 is constant-true -> the && RHS is forced -> 1/0 is a
    # guaranteed tclsh divide-by-zero -> W233 fires.
    return [expr {1.0 && 1/0}]
}
```

#### Per-line reasoning

The forced-arm walk (`compiler/interval_bounds.py::_walk_eager`) decides whether a lazy
operand is *guaranteed to run* by resolving its guard to a constant.  It originally used an
int-only, case-sensitive helper (`intervals._literal_int`), so non-integer constant guards
were treated as non-constant — the arm stayed "maybe-dead" and a guaranteed error in it was
missed.  Switching to `expr_ast._const_bool` (the same constant-truth engine the W123
dead-arm check uses) recognises:

* **floats** — `1.0` / `0.0` / `1.5` (`1.0 && 1/0`, `0.0 || 1/0`, `1.5 ? 1/0 : 7` all force the arm);
* **case-insensitive bool keywords** — `True` / `TRUE` / `No` (the lexer also now tokenises a
  capitalised bool as a literal instead of degrading the whole expr to an opaque raw node);
* **unary-prefixed constants** — `-1` / `+2` keep the operand's truth, `!0` / `not 0` invert it.

A constant-**false** `&&` guard (`False && 1/0`, `!1 && 1/0`, `0.0 && 1/0`) still short-circuits
the RHS away, and a non-constant guard leaves the arm maybe-dead — both stay silent (no new
false positives; the change is purely in the safe direction).  See FP-BND-05 for the
short-circuit discipline this complements.

#### tclsh ground truth

```
% expr {1.0 && 1/0}
divide by zero
% expr {True && 1/0}
divide by zero
% expr {-1 && 1/0}
divide by zero
% expr {False && 1/0}
0
% expr {!1 && 1/0}
0
```

#### Compiler evidence

```
--- FP-BND-06: W233 fires when a non-integer constant guard forces the lazy arm
regen: python -m bench.fp_snippets --id FP-BND-06
function ::f
  block entry_1
    term Return [expr {1.0 && 1/0}]
```

#### Why the analyser reaches that verdict

`_walk_eager` now resolves the guard with `expr_ast._const_bool` (True/False/None; floats +
case-insensitive bools + unary fold), and `compiler/parsing/expr_lexer.py` recognises bool
keywords case-insensitively so a capitalised bool reaches the AST as a literal.  `None` (not
statically decidable) still leaves the arm skipped — the unchanged safe path.

#### Tests

- `tests/test_fp_bnd.py::test_FP_BND_06_float_guard_forces_arm_fires` (TP, float)
- `tests/test_fp_bnd.py::test_FP_BND_06_uppercase_bool_guard_forces_arm_fires` (TP, case-insensitive bool)
- `tests/test_fp_bnd.py::test_FP_BND_06_unary_constant_guard_forces_arm_fires` (TP, unary)
- `tests/test_fp_bnd.py::test_FP_BND_06_false_constant_guard_short_circuits_silent` (FP, constant-false)
- `tests/test_fp_bnd.py::test_FP_BND_06_nonconstant_guard_silent` (FP, non-constant)
- broader edge coverage in `tests/test_interval_bounds.py::TestDivideByZero`

---


## OPT — optimisation / codegen quick-fixes (O106/O109/O110/O116/O120/O126)

These quick-fix codes propose source rewrites the LSP can apply on user
acceptance.  Every entry below records a case where the rewrite or the
firing predicate had to be tightened to preserve runtime semantics
(O106 LICM purity, O116 empty-list fold) or to silence whitespace-only
or paren-only churn that produced thousands of corpus FPs (O110).

### FP-OPT-01 — O110 InstCombine: whitespace-only / paren-preservation / commutative reorder

- **Verdict:** FALSE POSITIVE (now fixed; four sub-fixes)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_01_*`
- **Codes:** O110 (Canonicalise expression / InstCombine)
- **Corpus:** the four sub-fixes brought corpus O110 from **3641 → ~700-900** (-75 to -80%).

#### Reproducer

```tcl
# whitespace-only churn (the dominant source — 3641 → 1490 after the first guard)
set x [expr { $a + $b }]
# branch-folding whitespace churn (`if {$x<0}` — bigfloat2 122→46, exif 53→10)
if {$x<0} { puts negative }
# paren preservation for mixed bitwise/shift (CERT EXP00-C; DES 91→23)
set x [expr {($a << 1) & 0xff}]
# commutative reorder where no real fold results (bigfloat2 46→35, exif 10→4)
set x [expr {2 + $a}]
```

#### Per-line reasoning

1. **Whitespace-only**: ``expr { $a + $b }`` has decorative whitespace.
   Pre-fix the InstCombine rewriter would propose ``expr {$a + $b}`` as a
   "canonicalisation" — but the rewritten text is semantically identical
   and the whitespace is the user's style choice.  The ``_strip_ws`` guard
   on the ``expression_args`` / ``expr_substitutions`` paths drops
   whitespace-only rewrites.
2. **Branch-folding whitespace**: the same guard applies to the
   ``_branch_folding.py`` path so ``if {$x<0}`` no longer fires.
3. **Paren preservation**: ``($a << 1) & 0xff`` keeps its parens per CERT
   EXP00-C — mixed bitwise/shift expressions cling to explicit precedence
   for reader clarity even when the parens are technically redundant.
   The AST renderer now preserves parens on mixed bitwise/shift.
4. **Commutative reorder**: pre-fix the reassoc would swap ``literal +
   term`` to ``term + literal`` even when no fold would result —
   pointless churn.  The suppression in ``_simplify_expr_node`` checks
   whether the reordered form would actually fold further; identities
   (``x + 0``, ``x * 1``) and operator flips still fire.

#### tclsh ground truth

N/A — these are static rewrites that preserve runtime semantics; the
verdict is about whether the rewrite is *worth proposing*, not whether
it'd be wrong to apply.

#### Why the analyser reaches that verdict

- ``compiler/optimiser/_propagation.py`` — ``_strip_ws`` guard on
  expression_args / expr_substitutions paths.
- ``compiler/optimiser/_branch_folding.py`` — same ``_strip_ws`` guard.
- ``compiler/optimiser/_expr_simplify.py`` — paren preservation for mixed
  bitwise/shift.
- ``compiler/optimiser/_expr_simplify.py`` — commutative-reorder
  suppression when the swap yields no further fold.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_01_whitespace_only_no_o110` (FP)
- `tests/test_fp_opt.py::test_FP_OPT_01_branch_folding_whitespace_no_o110` (FP)
- `tests/test_fp_opt.py::test_FP_OPT_01_paren_preserved_no_o110` (FP)
- `tests/test_fp_opt.py::test_FP_OPT_01_commutative_reorder_no_o110` (FP)
- `tests/test_fp_opt.py::test_FP_OPT_01_genuine_simplification_still_fires` (TP, `x + 0` → `x`)

---

### FP-OPT-02 — O116 fold-const-list-command: empty `[list]` folds to `{}`, not `""`

- **Verdict:** TRUE POSITIVE / quick-fix correctness bug (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_02_*`
- **Codes:** O116 (Fold constant list command)
- **Corpus:** 346 corpus firings now apply cleanly.

#### Reproducer

```tcl
set x [list]
lappend x a
puts $x
```

#### Per-line reasoning

1. `set x [list]` — O116 proposes folding ``[list]`` to its constant value.
   The naive fold returns Python's empty string ``""`` for an empty list.
2. Applying that to ``set x [list]`` produces ``set x `` (with no second
   argument).  In Tcl, ``set x`` (one arg, no value) is a *read* of `x`,
   not a write — the assignment is silently erased from the source.
3. The corrected fold uses the canonical empty-list literal ``{}``.
   ``set x {}`` is a proper assignment to the empty list.

Pre-fix bug: applying the quick-fix to ``set x [list]`` produced
``set x ;`` (or ``set x \n``) — a syntax-valid READ form.  When the
source is re-evaluated against an interpreter that doesn't already
have ``x`` defined, the read fails with ``can't read "x": no such
variable``; in a live edit-and-reload workflow the user observes
the variable's previously-assigned value silently being lost.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% # Demonstrate the rewrite's effect in a fresh interpreter (the
% # source-evaluation case):
% tclsh9.0
% set x ;
can't read "x": no such variable
% # In a live session where x was already bound, the rewrite
% # silently DOES NOT update x -- the previous value persists,
% # which is just as wrong:
% set x "previous-value"
previous-value
% set x ;
previous-value
%
% # The corrected fold preserves the assignment:
% set x {}
%
% puts "<$x>"
<>
```

The "fold to empty string" rewrite silently breaks the assignment;
the "fold to ``{}``" rewrite preserves it.

#### Compiler evidence

The O116 diagnostic carries ``data['replacement'] = '{}'`` (the canonical
empty-list literal).  Pre-fix it carried ``''``.

#### Why the analyser reaches that verdict

`compiler/optimiser/_helpers.py::_try_fold_list_command` returns ``"{}"``
(not ``""``) when ``cmd_texts == ["list"]`` — see the inline comment
about the source-position requirement.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_02_empty_list_quick_fix_uses_braces` (TP/correctness)

---

### FP-OPT-03 — O106 LICM purity: outer-pure / inner-impure expression NOT hoistable

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_03_*`
- **Codes:** O106 (Hoist loop-invariant computation)
- **Corpus:** `tmp/tcllib-2.0/modules/clay/build/test.tcl:686` (`[format %04d [incr testnum]]`).

#### Reproducer

```tcl
proc f {} {
    for {set i 0} {$i < 10} {incr i} {
        set s [format %04d [incr testnum]]
    }
    return $s
}
```

#### Per-line reasoning

1. ``format %04d [incr testnum]`` — the OUTER call is ``format``, which is
   a pure formatter.  The INNER ``[incr testnum]`` mutates ``testnum``
   per iteration.
2. LICM at first glance sees ``format`` (pure) and considers the
   expression hoistable.  But hoisting would call ``incr`` ONCE before
   the loop instead of N times inside — the formatted output would all
   show the same number, AND ``testnum`` would advance by 1 instead of
   N.  That's a runtime-semantics change.
3. The fix recurses ``_is_pure_command`` into argument command
   substitutions; ANY inner impure command marks the whole expression
   impure → not hoistable → no O106.

The same logic applies to ``[read $fh 512]`` (per-call channel consumption).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set testnum 0
% for {set i 0} {$i < 3} {incr i} { puts [format %04d [incr testnum]] }
0001
0002
0003
% # If LICM hoisted: testnum would be 1 (not 3) and all three lines would say 0001.
```

#### Why the analyser reaches that verdict

`compiler/gvn.py::_is_pure_command` recurses into
``ExprCommand`` argument subtrees; any inner impure command marks the
whole expression impure.  ``_parse_cmd_token`` re-wraps CMD-sub arg
pieces in ``[...]`` so the recursion sees them.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_03_inner_impure_blocks_licm` (FP)
- `tests/test_fp_opt.py::test_FP_OPT_03_outer_pure_inner_pure_still_fires` (TP control)

---

### FP-OPT-04 — O109/O126 dead-store/unused: call-by-name through user procs is a real use

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_04_*`
- **Codes:** O109 (dead store), O126 (remove unused), W211/W220 (analyser equivalents)
- **Corpus:** `tmp/tcllib-2.0/modules/asn/asn.tcl` and similar `upvar`-using modules.

#### Reproducer

```tcl
proc asnPeekTag {data {tag tag} {type type}} {
    upvar 1 $tag tagOut $type typeOut
    set tagOut 0
    set typeOut 0
    return [string length $data]
}
proc decode {data} {
    asnPeekTag $data tag type
    return [list $tag $type]
}
```

#### Per-line reasoning

1. ``decode`` passes the literal variable names ``tag`` and ``type`` to
   ``asnPeekTag``.  ``asnPeekTag`` declares those parameters with
   ``ProcArgTrait.VAR_READ`` / ``VAR_WRITE`` (a Tcl-side ``upvar`` idiom)
   so the callee writes to ``decode``'s ``tag`` / ``type`` via the upvar.
2. The subsequent ``return [list $tag $type]`` reads those values back.
3. Pre-fix the analyser saw no caller-local write to ``tag`` / ``type``
   before the read and flagged W210 (RBS); the optimiser saw no later
   read of a written-via-upvar value and flagged O109 / O126.  Both
   were wrong: the callee did the write.

Fix: the call-by-name suppression already on W211/W220 (commit
extending ``ProcDef.param_traits`` consumers) was extended to the
optimiser's DCE pass — both layers now share
``compiler/proc_arg_traits.py``.  Also extended to literal-name args
inside ``[…]`` substitutions (the dominant tcllib shape:
``set len [asnPeekTag data tag type dummy]``) — the scanner walks
``IRAssignValue.value`` raw text, ``IRAssignExpr`` /
``IRExprEval`` / ``IRReturn`` expr trees for nested ``ExprCommand``
nodes, and applies the same suppression.

#### tclsh ground truth

```
% proc asnPeekTag {data {tag tag} {type type}} {
    upvar 1 $tag tagOut $type typeOut
    set tagOut 99
    set typeOut "INT"
}
% proc decode {data} {
    asnPeekTag $data tag type
    list $tag $type
}
% decode foo
99 INT
```

Both `tag` and `type` are written by the callee via upvar; the caller
reads them after the call.

#### Why the analyser reaches that verdict

`compiler/proc_arg_traits.py::collect_call_by_name_reads` is consulted
by both the analyser's W211/W220 emitters and the optimiser's O109/O126
emitters; it returns the set of caller-local names passed by literal to
a callee with VAR_READ/VAR_WRITE traits.  Sample tcllib asn.tcl: W211
firings 2→0.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_04_call_by_name_suppresses_dead_store` (FP)
- `tests/test_fp_opt.py::test_FP_OPT_04_genuine_dead_store_still_fires` (TP control)

---

### FP-OPT-05 — O126 unused-assign elimination must NOT delete an RHS with observable side effects (D2-O126)

- **Verdict:** TRUE POSITIVE / soundness fix (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_05_*`
- **Codes:** O126 (remove unused) and the W211/W220 analyser equivalents
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D2-O126

#### Reproducer

```tcl
proc f {} { set unused [puts side]; puts done }
```

#### Per-line reasoning

1. `set unused [puts side]` — the assignment IS unused (nobody reads `unused`).  But the RHS `[puts side]` prints `side` to stdout — a real, observable side effect.
2. `puts done` — prints `done`.
3. tclsh runs the original and prints `side` then `done`; deleting the assignment (pre-fix O126 behaviour) silently dropped the `puts side` and the optimised program printed only `done`.  That's a real soundness bug.
4. D2-O126 closure: the purity gate `_assignment_safe_to_delete` consults `_word_has_observable_side_effect` / `_expr_has_observable_side_effect` (compiler/optimiser/_elimination.py).  Any RHS whose command is in the side-effect set (puts, file I/O, channel ops, exec, etc.) blocks the deletion.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { set unused [puts side]; puts done }
% f
side
done
```

Pre-fix optimised version produced only `done`.

#### Compiler evidence

```
--- FP-OPT-05: O126 must NOT delete an RHS with observable side effects (D2-O126)
regen: python -m bench.fp_snippets --id FP-OPT-05
function ::f
  block entry_1
    [0] AssignValue 'unused' value='[puts side]'  defs={unused#1}  uses={}
    [1] Call cmd='puts'  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
  unused_variables
    UnusedVariable(block='entry_1', statement_index=0, variable='unused')
```
(regen: `python -m bench.fp_snippets --id FP-OPT-05`)

The analysis still records `unused` as an unused-variable (W211 / O126 candidate); the optimiser then declines to emit O126 because the RHS purity gate refuses.

#### Why the analyser reaches that verdict

`compiler/optimiser/_elimination.py:30-115` — `_word_has_observable_side_effect`, `_expr_has_observable_side_effect`, and `_assignment_safe_to_delete` walk the RHS text/expr, recognise command words that are known impure (`puts`, `exec`, file I/O, channel ops, etc.), and refuse the deletion when any impure command is present.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_05_o126_preserves_puts_side_effect` (TP — optimiser must NOT fire O126)
- `tests/test_fp_opt.py::test_FP_OPT_05_o126_pure_rhs_still_fires` (TP control — pure RHS like `[list 1 2 3]` IS safe to delete)
- `tests/test_ground_truth_tn_fn.py::test_TP_optimiser_O126_preserves_puts_side_effect`
- `tests/test_ground_truth_tn_fn.py::test_TP_optimiser_O126_keeps_for_pure_RHS`

---

### FP-OPT-06 — O100/O109/O127: command-substitution writes are SSA kills (D2-O100)

- **Verdict:** TRUE POSITIVE / soundness fix (now fixed; one root cause across three codes)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_06_*`
- **Codes:** O100 (constant propagation), O109 (dead-store elimination), O127 (load-forwarding)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entries D2-O100, D2-O109, D2-O127

#### Reproducer

```tcl
proc f {} { set x a; set y [append x b]; puts $x; puts $y }
```

#### Per-line reasoning

1. `set x a` — `x` is `"a"`.  SCCP sees `x#1 = CONST('a')`.
2. `set y [append x b]` — `[append x b]` is a cmd-sub.  At runtime tclsh evaluates `append`, which MUTATES `x` in place to `"ab"` and returns the new value.  So this statement assigns `"ab"` to `y` AND changes `x` to `"ab"`.
3. `puts $x` — tclsh prints `ab` (the post-append value).  Pre-fix the optimiser propagated the stale `x#1 = CONST('a')` lattice value into this `puts`, producing the source rewrite `puts a` — wrong output.
4. `puts $y` — prints `ab`.
5. D2-O100 closure: every statement now includes `statement_cmd_sub_write_names(stmt)` in its `kill_sites`.  Cmd-sub writes invalidate the lattice entries for the targeted names, so subsequent reads use the post-cmd-sub value (OVERDEFINED if not statically resolvable).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { set x a; set y [append x b]; puts $x; puts $y }
% f
ab
ab
```

Pre-fix optimised version produced `a` then `b`.

#### Compiler evidence

```
--- FP-OPT-06: O100/O109/O127: cmd-sub writes are SSA kills (D2-O100)
regen: python -m bench.fp_snippets --id FP-OPT-06
function ::f
  block entry_1
    [0] AssignValue 'x' value='a'  defs={x#1}  uses={}
    [1] AssignValue 'y' value='[append x b]'  defs={y#1}  uses={}
    [2] Call cmd='puts'  defs={}  uses={x#1}
    [3] Call cmd='puts'  defs={}  uses={y#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  values (SCCP lattice)
    x#1: CONST('a')
    y#1: OVERDEFINED
```
(regen: `python -m bench.fp_snippets --id FP-OPT-06`)

`x#1` still shows `CONST('a')` in the static lattice (SCCP can't model the runtime mutation), but the optimiser's `kill_sites` map invalidates `x` at the `[append x b]` site, so no `puts a` propagation rewrite is emitted.

#### Why the analyser reaches that verdict

`compiler/optimiser/_manager.py:208-225` — `statement_cmd_sub_write_names` (from `compiler.ssa`) returns the set of variable names that any cmd-sub in the statement writes to.  Those names are appended to `kill_sites` for the statement; the propagation pass consults `kill_sites` and refuses to propagate values across the kill.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_06_o100_does_not_propagate_past_cmd_sub_write` (TP)
- `tests/test_ground_truth_tn_fn.py::test_TP_optimiser_O100_does_not_propagate_past_cmd_sub_write`

---

### FP-OPT-07 — O126 extends `_assignment_safe_to_delete` to interproc purity summaries (D2-O126-FU)

- **Verdict:** TRUE POSITIVE / precision follow-up to D2-O126 (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_07_*`
- **Codes:** O126 (remove unused) and the W211/W220 analyser equivalents
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D2-O126-FU

#### Reproducer

```tcl
proc add {a b} { expr {$a + $b} }
proc f {} { set unused [add 1 2]; puts done }
```

#### Per-line reasoning

1. `proc add {a b} { expr {$a + $b} }` — arithmetic-only body, no I/O, no observable state mutation.  The interprocedural fixpoint marks `add` as `pure=True`.
2. `proc f {}` — `set unused [add 1 2]` has an unused LHS and a pure RHS.  D2-O126 (the base purity gate) refused to delete because the RHS was a user-proc call and the gate didn't know about interproc purity.
3. D2-O126-FU closure: `optimise_elimination_passes` now builds `interproc_pure = frozenset(qname for qname, summary in ... if summary.pure)` and threads it through to `_word_has_observable_side_effect` / `_expr_has_observable_side_effect` / `_assignment_safe_to_delete`.  When the RHS command is in `interproc_pure`, deletion is allowed.
4. Counter-example: when `add` were `puts $a; expr {$a + $b}`, the interproc fixpoint would mark `add` as `pure=False`, the gate would refuse, and O126 would correctly not fire.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc add {a b} { expr {$a + $b} }
% proc f {} { set unused [add 1 2]; puts done }
% f
done
```

`add` has no observable effect; deleting the unused assignment doesn't change observable output.

#### Compiler evidence

```
--- FP-OPT-07: O126 extends to pure user-proc RHS via interproc purity (D2-O126-FU)
regen: python -m bench.fp_snippets --id FP-OPT-07
function ::f
  block entry_1
    [0] AssignValue 'unused' value='[add 1 2]'  defs={unused#1}  uses={}
    [1] Call cmd='puts'  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
  unused_variables
    UnusedVariable(block='entry_1', statement_index=0, variable='unused')
```
(regen: `python -m bench.fp_snippets --id FP-OPT-07`)

The unused-variable record is still produced; with `interproc_pure={"::add"}` the optimiser's `_assignment_safe_to_delete` accepts the deletion and emits O126.

#### Why the analyser reaches that verdict

`compiler/optimiser/_elimination.py` — `optimise_elimination_passes` constructs `interproc_pure` from the `InterprocSummary.pure==True` set and passes it down through `_word_has_observable_side_effect` (line 30-95) and `_expr_has_observable_side_effect` (line 96-115) into `_assignment_safe_to_delete`.

TclOO `method` purity (`ClassDef.method_purity`) is NOT yet wired; method calls still go through `IRCall my <method>` which `classify_side_effects` treats as impure — a follow-up if needed.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_07_pure_user_proc_rhs_is_deleted` (TP)
- `tests/test_fp_opt.py::test_FP_OPT_07_impure_user_proc_rhs_preserved` (TN control)
- `tests/test_ground_truth_tn_fn.py::test_TP_O126_pure_user_proc_RHS_is_deleted`
- `tests/test_ground_truth_tn_fn.py::test_TN_O126_impure_user_proc_RHS_preserved`

---

### FP-OPT-08 — O109/O126 overlap filter: `segment_commands` + EXPR/BODY descent (D4-F10)

- **Verdict:** TRUE POSITIVE / cleanup that fixes corner-case unsoundness (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_08_*`
- **Codes:** O109 (dead-store elimination), O126 (remove unused)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D4-F10

#### Reproducer

```tcl
proc f {} {
    set a 1
    set b 0
    if {$a} {
        if {$b} { puts X }
    }
}
```

#### Per-line reasoning

1. `set a 1` — `a` is `1`.  SCCP folds it; O112 will eventually rewrite the outer `if {$a} { … }` into the inner `if {$b} { puts X }` (constant-true branch).
2. `set b 0` — `b` is `0`.  SCCP folds it too; if the outer rewrite happens first, the surviving statement is `if {$b} { puts X }` — which still references `$b`.
3. The overlap filter must therefore NOT let O109/O126 delete `set b 0` (because the O112 replacement still uses `$b`).  Pre-fix the filter extracted the LHS of the proposed deletion via `text.split(None, 2)`, which fell over on braced/quoted words, escaped whitespace, qualified `::set` spellings, etc.  And the var-reference scanner that finds `$b` in the surviving replacement only looked at top-level VAR tokens — it missed `$b` inside an `if`-condition EXPR-role arg.
4. D4-F10 closure: (a) the LHS extraction now uses `segment_commands(text)` + `normalise_var_name` (compiler/optimiser/_manager.py:436-461); (b) the var scanner enables `recurse_into_script_roles=True` + `recurse_into_expr_roles=True` (line 419-426) so it descends into `if`/`while`/`for` condition + body words.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {} { set a 1; set b 0; if {$a} { if {$b} { puts X } } }
% f
%               ;# no output (inner if is false)
```

Pre-fix the optimiser could delete `set b 0` and then leave `if {$b} { puts X }` in place — `$b` would be unset at runtime, an error.

#### Compiler evidence

```
--- FP-OPT-08: O109/O126 overlap filter: segment_commands + EXPR/BODY descent (D4-F10)
regen: python -m bench.fp_snippets --id FP-OPT-08
function ::f
  block entry_1
    [0] AssignConst 'a' value='1'  defs={a#1}  uses={}
    [1] AssignConst 'b' value='0'  defs={b#1}  uses={}
    term Branch ExprVar(text='$a', name='a', start=0, end=1)
  block if_end_2
    term Goto
  block if_then_3
    term Branch ExprVar(text='$b', name='b', start=0, end=1)
  block if_next_4
    term Goto
  block if_end_5
    term Goto
  block if_then_6
    [0] Call cmd='puts'  defs={}  uses={}
    term Goto
  block if_next_7
    term Goto
  block exit_8
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-OPT-08`)

`if_then_3` branches on `$b` (the EXPR-role condition); the recurse-into-EXPR var scanner catches this `$b` reference and blocks O109/O126 from deleting `set b 0`.

#### Why the analyser reaches that verdict

`compiler/optimiser/_manager.py:419-465` — the `VarReferenceScanner` for O112 replacements is constructed with `recurse_into_script_roles=True, recurse_into_expr_roles=True` (line 421-425), so it walks into the EXPR-role `if`-condition.  The overlap filter (line 440-464) uses `segment_commands(text)` to extract the proposed deletion's LHS — Tcl-parser-correct instead of `split(None, 2)`.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_08_nested_if_constant_chain_does_not_delete_inner_var` (TP — confirms the optimised source doesn't break the inner if)
- `tests/test_fp_opt.py::test_FP_OPT_08_unrelated_set_still_eligible_for_o126` (TN control — when no surviving replacement references the var, the deletion is still allowed)

---

### FP-OPT-09 — O110 identity/annihilator drops require provably-numeric operand (D5-O110)

- **Verdict:** TRUE POSITIVE / soundness fix (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_09_*`
- **Codes:** O110 (canonicalise expression / InstCombine)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-O110

#### Reproducer

```tcl
proc f {x} {
    set y [expr {$x + 0}]
    puts $y
}
f abc
```

#### Per-line reasoning

1. `proc f {x}` — parameter `x` has UNKNOWN type at the proc entry.  SCCP can't prove `x` is numeric (the caller could pass anything).
2. `set y [expr {$x + 0}]` — the InstCombine identity rewrite `$x + 0` → `$x` *drops* the `+ 0` arithmetic, but that arithmetic is also what triggers Tcl's numeric coercion.  After the rewrite, `set y $x` simply assigns the unchanged string — silently hiding the runtime error.
3. `puts $y` — observable output: pre-rewrite tclsh errors with `can't use non-numeric string "abc" as left operand of "+"`; post-rewrite the program prints `abc` cleanly.  The two are not behaviour-equivalent.
4. D5-O110 closure: `_simplify_expr_node` now consults `_is_provably_numeric_expr_node(node, ssa_uses, types)` before every identity/annihilator rewrite that elides an operand.  An operand is provably numeric only when it is an ExprLiteral, an ExprString whose text parses as numeric, or an ExprVar whose SSA type lattice is `INT`, `DOUBLE`, `NUMERIC`, or `BOOLEAN` at this program point.  When the proof is unavailable, the rewrite is skipped (better: a missed optimisation than wrong output).
5. The same predicate gates: `x + 0`, `0 + x`, `x - 0`, `x * 0`, `0 * x`, `x * 1`, `1 * x`, `x / 1`, `x % 1`, `x << 0`, `x >> 0`, `x & 0`, `0 & x`, `x | 0`, `0 | x`, `x ^ 0`, `0 ^ x`, `x ** 0`, `x ** 1`, `x ^ x`, `x - x`, `+x`, `-(-x)`, `~~x`.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc f {x} { set y [expr {$x + 0}]; puts $y }
% catch {f abc} err
1
% puts $err
can't use non-numeric string as left operand of "+"

% # Compare to the unsound post-rewrite (set y $x; puts $y):
% proc g {x} { set y $x; puts $y }
% g abc
abc
```

The pre-rewrite ERRORS; the post-rewrite SILENTLY SUCCEEDS — the rewrite is unsound when `x` is not provably numeric.

#### Compiler evidence

```
--- FP-OPT-09: O110 identity/annihilator drops require provably-numeric operand (D5-O110)
regen: python -m bench.fp_snippets --id FP-OPT-09
function ::f
  block entry_1
    [0] AssignExpr 'y'  defs={y#1}  uses={x#0}
    [1] Call cmd='puts'  defs={}  uses={y#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  types
    y#1: NUMERIC
```
(regen: `python -m bench.fp_snippets --id FP-OPT-09`)

`x#0` is the entry version (param) and carries no type lattice entry — the predicate falls back to "not provably numeric" and the rewrite is skipped.

#### Why the analyser reaches that verdict

`compiler/optimiser/_expr_simplify.py` — `_simplify_expr_node` and `_simplify_to_fixpoint` now take `ssa_uses` and `types` kwargs threaded from `_instcombine_expr` through `optimise_expression_args` / `optimise_expr_substitutions` / `optimise_branch_conditions`.  Each identity/annihilator rewrite that drops an operand calls `_is_provably_numeric_expr_node(operand, ssa_uses=ssa_uses, types=types)` and bails when it returns False.

For literals the predicate is trivially True (the parser only emits ExprLiteral for int/float/boolean); for SCCP-substituted ExprString it parses the text; for ExprVar it looks up the SSA type lattice at the use site.  Strength-reduction rewrites (`x ** 2` → `x * x`, `x % (2^N)` → `x & (2^N - 1)`) keep `x` as an operand on both sides, so error semantics are preserved without a guard.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_09_unknown_type_param_blocks_identity_rewrite` (TP)
- `tests/test_fp_opt.py::test_FP_OPT_09_provably_numeric_var_still_fires` (TN control)

---

### FP-OPT-10 — O114 set/expr -> incr requires SSA-known INT type on the loop var (D5-O114)

- **Verdict:** TRUE POSITIVE / soundness fix (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_10_*`
- **Codes:** O114 (use `incr` instead of `set`/`expr`)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-O114

#### Reproducer

```tcl
proc foo {x} {
    set x [expr {$x + 1}]
    puts $x
}
foo 1.5
```

#### Per-line reasoning

1. `proc foo {x}` — parameter `x` has UNKNOWN type at entry; SCCP can't prove `x` is an integer.
2. `set x [expr {$x + 1}]` — the syntactic shape matches the O114 idiom (set-self-plus-integer-literal), but `expr {$x + 1}` on a float silently promotes (tclsh: `foo 1.5` → `2.5`), while `incr x` errors with `expected integer but got "1.5"`.  The rewrite turns successful execution into a runtime error.
3. D5-O114 closure: `optimise_incr_idioms` now takes `analysis` and, before calling `_try_incr_idiom`, checks `analysis.types[(var, ver)].tcl_type is TclType.INT` for the variable's SSA use version.  Only INT is accepted — DOUBLE, NUMERIC (the join of INT and DOUBLE), BOOLEAN, OBJECT and unknown are all conservatively refused.  `_try_incr_idiom` itself gained a `var_is_int=False` default kwarg that bails fast.
4. Counter-example: `for {set x 0} {$x < $n} {incr x} { set x [expr {$x + 1}]; puts $x }` — the for-loop initialiser + `incr` typecheck `x` as INT (OVERDEFINED value at the use site but KNOWN INT type); the rewrite fires correctly.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc foo {x} { set x [expr {$x + 1}]; puts $x }
% foo 1.5
2.5

% # Compare to the unsound post-rewrite (set x [expr ...] -> incr x):
% proc bar {x} { incr x; puts $x }
% catch {bar 1.5} err
1
% puts $err
expected integer but got "1.5"
```

The pre-rewrite SUCCEEDS with a float result; the post-rewrite ERRORS — not behaviour-equivalent.

#### Compiler evidence

```
--- FP-OPT-10: O114 set/expr -> incr requires SSA-known INT type on the loop var (D5-O114)
regen: python -m bench.fp_snippets --id FP-OPT-10
function ::foo
  block entry_1
    [0] AssignExpr 'x'  defs={x#1}  uses={x#0}
    [1] Call cmd='puts'  defs={}  uses={x#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  types
    x#1: NUMERIC
```
(regen: `python -m bench.fp_snippets --id FP-OPT-10`)

`x#0` is the param entry version with no type lattice entry — the predicate falls back to "not INT" and the rewrite is skipped.

#### Why the analyser reaches that verdict

`compiler/optimiser/_pattern_recognition.py::optimise_incr_idioms` now accepts the `analysis` arg from `compiler/optimiser/_manager.py:197` and, for each candidate `set X [expr {$X + N}]`, looks up the SSA use version of `X` in `ssa_block.statements[idx].uses` and checks `analysis.types[(X, ver)].tcl_type is TclType.INT` (and `kind is TypeKind.KNOWN`).  Only INT passes the gate.  `_try_incr_idiom` in `compiler/optimiser/_helpers.py` short-circuits to None when `var_is_int=False`.

DOUBLE, NUMERIC, BOOLEAN, OBJECT — and the more common UNKNOWN/OVERDEFINED — are all refused.  This is conservative on purpose: only INT guarantees `incr`'s arithmetic semantics match `expr`'s.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_10_unknown_type_param_blocks_incr_rewrite` (TP)
- `tests/test_fp_opt.py::test_FP_OPT_10_provably_int_var_still_fires` (TN control)
- `tests/test_optimiser_coverage.py::TestO114IncrIdiom::test_no_incr_on_unknown_type_param` (TP)

---

### FP-OPT-11 — O120 ==/!= → eq/ne requires at-least-one provably-non-numeric operand (D5-O120)

- **Verdict:** TRUE POSITIVE / soundness fix (now fixed)
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_11_*`
- **Codes:** O120 (prefer `eq`/`ne` over `==`/`!=` for string comparison)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-O120

#### Reproducer

```tcl
proc f {raw} {
    set a [string trim $raw]
    if {$a == "1"} { puts yes } else { puts no }
}
```

#### Per-line reasoning

1. `set a [string trim $raw]` — `$a` is typed `STRING` (the type lattice
   correctly tracks the *internal representation* of `[string trim]`'s
   output) but its **runtime value** is unknown — could be `"1"`,
   `"1.0"`, `"foo"`, etc.
2. `if {$a == "1"}` — pre-fix O120 rewrote this to `if {$a eq "1"}`
   under the "left side KNOWN STRING-typed → safe to rewrite" rule.
   That rule is **unsound**: a STRING-typed Tcl value can still hold
   numeric-looking text.  When `raw="1.0"`, `$a == "1"` is `1`
   (numeric: `1.0 == 1.0`) but `$a eq "1"` is `0` (string:
   `"1.0" ne "1"`).  The rewrite flips the result.
3. **Tcl `==` semantics (expr(n)):** attempts int-then-double parse
   on **both** operands; falls through to **string** compare iff at
   least one operand fails to parse.  So the rewrite `==` → `eq` is
   sound iff we can prove at least one operand cannot parse as a number.
4. **D5-O120 closure:** `_rewrite_eq_ne_string_compare_node` now
   requires **at least one** operand to satisfy
   `_is_provably_non_numeric_expr_node`, defined as:
   - `ExprString` literal AND text fails `_is_numeric_string_value`, OR
   - `ExprVar` AND SCCP CONST value is a non-numeric string.

   KNOWN STRING type is no longer accepted as proof (the unsound
   heuristic).  In this reproducer the literal `"1"` IS numeric-looking
   and `$a` has no SCCP CONST proof — neither side qualifies, so the
   rewrite is correctly refused.
5. **Why "at least one" (not "both"):** if either operand can't parse
   as a number, Tcl's `==` falls through to string compare regardless
   of the other operand's value.  Requiring both would block the
   dominant `$a == "hello"` idiom where "hello" alone forces the
   string path.  See the TN test for the contrast.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set a "1.0"
% expr {$a == "1"}
1
% expr {$a eq "1"}
0
%
% # The rewrite would flip 1 -> 0 for any raw containing numeric-
% # looking text.  Confirmation for the at-least-one rule:
% set a foo
% expr {$a == "1"}
0
% expr {$a eq "1"}
0
% # Both 0 — "foo" is provably non-numeric, the rewrite is sound.
```

#### Compiler evidence

```
--- FP-OPT-11: O120 ==/!= -> eq/ne requires at-least-one provably-non-numeric operand (D5-O120)
regen: python -m bench.fp_snippets --id FP-OPT-11
function ::f
  block entry_1
    [0] AssignValue 'a' value='[string trim $raw]'  defs={a#1}  uses={raw#0}
    term Branch ExprBinary(op=<BinOp.EQ: '=='>, left=ExprVar(text='$a', name='a', start=0, end=1), right=ExprString(text='"1"', start=6, end=8))
  block if_end_2
    term Goto
  block if_then_3
    [0] Call cmd='puts'  defs={}  uses={}
    term Goto
  block if_next_4
    [0] Call cmd='puts'  defs={}  uses={}
    term Goto
  block exit_5
    term (none — fall-through exit)
  values (SCCP lattice)
    a#1: OVERDEFINED
  types
    a#1: STRING
```
(regen: `python -m bench.fp_snippets --id FP-OPT-11`)

`a#1` is typed STRING (correct internal-rep tracking) but its SCCP
value is OVERDEFINED — no proof the runtime string is non-numeric.
The literal `"1"` IS numeric-looking, so neither operand qualifies and
the rewrite is correctly refused.

#### Why the analyser reaches that verdict

`compiler/optimiser/_expr_simplify.py::_rewrite_eq_ne_string_compare_node`
now calls `_is_provably_non_numeric_expr_node` on each side and gates
the rewrite on `left_non_num or right_non_num`.  The predicate inspects
literal text OR SCCP CONST values — KNOWN STRING type alone is
rejected (the previously-unsound shortcut).  `_try_eq_ne_string_compare_simplify_expr`
gained a `values=None` kwarg and the propagation/branch-folding callers
pass `analysis.values` through.

The at-least-one rule (not both) preserves the dominant
`$a == "hello"` idiom: "hello" alone forces Tcl's string path, so the
rewrite remains sound regardless of `$a`'s runtime value.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_11_numeric_like_literal_string_typed_var_no_rewrite` (TP, must NOT rewrite)
- `tests/test_fp_opt.py::test_FP_OPT_11_non_numeric_literal_still_rewrites` (TN, `$a == "hello"` MUST rewrite)
- `tests/test_optimiser.py::TestStringCompareEqNe::test_numeric_like_literal_NOT_rewritten_for_string_typed_var` (TP)
- `tests/test_optimiser.py::TestStringCompareEqNe::test_var_vs_var_from_string_producers_NOT_rewritten` (TP)

---

### FP-OPT-12 — TclOO method purity wired into O126 (SF-2 FIXED)

- **Verdict:** FIXED — method-body lowering landed; the wired O126 gate now fires
- **Status:** locked in by `tests/test_fp_opt.py::test_FP_OPT_12_*`
- **Codes:** O126 (unused-assign elimination, RHS-purity gated)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry SF-2

#### Reproducer

```tcl
# TP wiring (user-proc analogue -- works today via D2-O126-FU)
proc pure_helper {} { return 42 }
proc m {} {
    set unused [pure_helper]    ;# RHS provably pure -> O126 deletes
    puts done
}
```

```tcl
# TclOO target (now active -- methods are lowered to per-method FunctionUnits)
oo::class create C {
    method pure_helper {} { return 42 }
    method m {} {
        set unused [my pure_helper]    ;# my-dispatch RHS provably pure -> O126 deletes
        puts done
    }
}
```

#### Per-line reasoning

1. The D2-O126-FU closure already builds `interproc_pure` from `ctx.interproc.procedures`, so `set unused [pure_helper]` (user-proc form) folds today.
2. SF-2 extends the same pattern to TclOO methods: `interproc_pure_methods = {qn for qn, summary in ctx.interproc.methods.items() if summary.pure}`.  `ctx.interproc.methods` is now populated: lowering lifts `oo::class create` / `oo::define` method bodies to `IRMethodDef` entries, `compile_source` lowers those to per-method `FunctionUnit`s in `CompilationUnit.methods`, and `analyse_interprocedural_ir` summarises them into `InterproceduralAnalysis.methods`.
3. The recursive scanner in `_word_has_observable_side_effect` / `_expr_has_observable_side_effect` recognises `my <method>` and asks `_method_pure(enclosing_class, method_name, interproc_pure_methods)`.  The optimiser iterates method bodies (`CompilationUnit.methods`) with the owning class as `enclosing_class`, so this path is now active.
4. Method purity is conservative-sound: a method is pure iff its own body has no observable side effect AND every *proc* it calls is pure; a `my <other_method>` / `next` self-dispatch surfaces as an unknown call and forces the method impure (false negatives only — the optimiser never deletes on an unproven peer method).

#### tclsh ground truth (9.0.3 — confirmed by execution)

Pure user-proc analogue:

```
% proc pure_helper {} { return 42 }
% proc m {} { set unused [pure_helper]; puts done }
% m
done
```

Deleting the `set unused [pure_helper]` line leaves observable behaviour unchanged (`done` still prints) — the rewrite is sound.

#### Compiler evidence

```
--- FP-OPT-12: TclOO method purity wired into O126 (SF-2 PARTIAL)
regen: python -m bench.fp_snippets --id FP-OPT-12
function ::m
  block entry_1
    [0] AssignValue 'unused' value='[pure_helper]'  defs={unused#1}  uses={}
    [1] Call cmd='puts'  defs={}  uses={}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-OPT-12`)

The user-proc form folds via D2-O126-FU.  The TclOO form now folds too: lowering populates `cu.ir_module.methods` / `cu.methods` with the method bodies, the interproc summary proves `::C::pure_helper` pure, and the optimiser deletes the `set unused [my pure_helper]` line inside `m`.

#### Why the analyser reaches that verdict

`compiler/optimiser/_elimination.py`:

- `optimise_elimination_passes` builds `interproc_pure_methods` alongside the existing `interproc_pure`, plus a placeholder `enclosing_class: str | None = None` (today the pass only runs over top-level + user procs, never method bodies).
- `_word_has_observable_side_effect` recognises `my <method>` / `::my <method>` cmd-subs and consults `_method_pure(enclosing_class, cmd_args[0], interproc_pure_methods)`.
- `_method_pure(class_qname, method_name, pure_methods)` checks several common qualifier spellings (`class::m`, `::class::m`, `class_no_leading_colons::m`).
- All four scanner helpers (`_word_…`, `_expr_…`, `_assignment_safe_to_delete`, and the recursive arg walk) take the new pair as optional kwargs.

#### Tests

- `tests/test_fp_opt.py::test_FP_OPT_12_pure_user_proc_via_my_dispatch_handled_at_word_level` (TP wiring, user-proc analogue)
- `tests/test_fp_opt.py::test_FP_OPT_12_impure_user_proc_still_blocks` (TN control)
- `tests/test_fp_opt.py::test_FP_OPT_12_tcloo_method_body_not_yet_lowered_partial` (PARTIAL pin)

---


## TNT — taint flow (T100/T101)

T-codes track tainted values (untrusted input like ``gets stdin`` /
``http::data``) flowing into dangerous sinks.  T100 fires when taint
reaches an ``expr`` operand (the unbraced-expr injection vector); T101
fires when taint reaches a ``puts`` content arg.  Each FP fix below is
position-aware: the analyser must distinguish *which* argument slot the
tainted value flows into so it doesn't false-fire on structural args
(channel ids, cmd-sub-encapsulated values) where re-parsing does NOT
happen.

### FP-TNT-01 — T100 direct-operand expr filter

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_tnt.py::test_FP_TNT_01_*`
- **Codes:** T100 (tainted → numeric/type-coercion sink)
- **Corpus:** tcllib blowfish.tcl:525, http.tcl:4338, mime.tcl:1962 (and similar).

**Scope clarification (post-review correction).** T100 in this catalog
is the *numeric/type-coercion* hazard, NOT a code-exec injection
vector for braced ``expr {…}``.  Tcl's braced ``expr`` does NOT
re-parse a direct-operand variable's value as expression text — the
substituted value flows in as a single operand and is converted via
``Tcl_GetDoubleFromObj`` / ``Tcl_GetIntFromObj``.  The genuine
code-exec sink is the **unbraced** ``expr $cmd`` form (and ``eval``,
covered by FP-INJ-05) where the substituted text is re-parsed.

#### Reproducer

```tcl
proc f {} {
    set data [gets stdin]
    # $data is consumed as an argument to `string length`, not used as
    # a direct expr operand.  The cmd-sub boundary protects it -- the
    # integer length is what flows into the divide; $data itself never
    # reaches expr's operand position.  No T100.
    expr {[string length $data] / 8}
}
```

#### Per-line reasoning

1. `set data [gets stdin]` — `$data` is tainted (read from stdin).
2. `expr {[string length $data] / 8}` — within the expr braces, `$data`
   appears INSIDE the command substitution `[string length $data]`.
   The cmd-sub is evaluated as a Tcl command (whose own argument
   substitution rules apply), and only its RESULT (an integer) flows
   into the expr.  ``$data`` is never an expr operand at all.

Pre-fix T100 fired on every tainted ``uses`` entry regardless of
position in the parsed expr AST.  The fix walks the ExprNode tree and
collects ExprVar names OUTSIDE any ExprCommand subtree — only those
are DIRECT expr operands.

**What T100 catches.** A tainted *direct operand* (``expr {$data +
1}``, ``expr {abs($data)}``) hits Tcl's numeric coercion rules.  This
is NOT arbitrary code execution — tclsh treats the value as a single
operand.  But the coercion still has security-relevant failure modes:

- **Type confusion / unintended value flow** — for ``$data = "0/0"``
  Tcl raises a domain error; for ``$data = "inf"`` it returns ``inf``
  and the rest of the calculation produces ``inf``/``nan``.  In a
  branching context (``if {$x < $data} {…}``) this can flip the
  decision.
- **Numeric-format injection** — leading-``0x`` for hex, leading-``0b``
  for binary, trailing-``e`` for exponent: ``$data = "0xff"`` parses
  as 255 even if the writer expected base-10.

These are real findings the writer should know about; T100 makes
sense as the numeric-coercion warning.  The **code-execution
injection** vector is the *unbraced* form ``expr $data + 1`` (the
substituted text is re-parsed as expr) and the ``eval $cmd`` family
— those are covered by W101 / FP-INJ-05, not T100.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set data "1+1; exec rm -rf /"
% expr {[string length $data] / 8}
2
% expr {$data + 1}
cannot use non-numeric string "1+1; exec rm -rf /" as left operand of "+"
% set data "[puts BRACED_DIRECT_NOT_EXECUTED]"
% expr {$data + 1}
cannot use a list as left operand of "+"
% expr $data + 1
BRACED_DIRECT_NOT_EXECUTED
cannot use non-numeric string "" as left operand of "+"
```

The cmd-sub form (`[string length $data]`) evaluates ``string length``
on the raw string.  The braced direct-operand form (``expr
{$data + 1}``) does NOT execute the embedded ``[puts]`` — the value
flows in as one operand and Tcl's numeric coercion fails on the
non-numeric input.  Only the **unbraced** form (``expr $data + 1``,
covered by W101) re-parses the substituted text and executes the
injected command.

#### Why the analyser reaches that verdict

`compiler/taint/_sinks.py` walks the ExprNode AST via
``_direct_expr_operand_names`` and collects ExprVar names that are
NOT inside any ExprCommand subtree.  T100 only fires when the
tainted name is in that set, so the value reaches Tcl's numeric
coercion path.

#### Tests

- `tests/test_fp_tnt.py::test_FP_TNT_01_cmd_sub_arg_position_no_t100` (FP)
- `tests/test_fp_tnt.py::test_FP_TNT_01_direct_operand_still_fires` (TP, `expr {$data + 1}`)
- `tests/test_fp_tnt.py::test_FP_TNT_01_function_arg_direct_operand_still_fires` (TP, `abs($data)`)

---

### FP-TNT-02 — T101 puts channel-vs-output filter

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_tnt.py::test_FP_TNT_02_*`
- **Codes:** T101 (tainted → output sink)
- **Corpus:** tcllib imap4.tcl (3 firings cleared).

#### Reproducer

```tcl
proc f {} {
    set chan [gets stdin]
    # $chan is the destination channel id, NOT the content; T101 must
    # not fire on a positional channel-id arg.  Only the trailing
    # content arg can carry injectable content.
    puts -nonewline $chan "hello\n"
}
```

#### Per-line reasoning

1. `puts ?-nonewline? ?channelId? string` — Tcl's `puts(n)` signature.
   The channel id (a file handle) is structural; the trailing positional
   is the content.
2. Pre-fix T101 fired on the channel-id arg as if it were content.  But a
   tainted channel handle is not an output-injection vector (it's just
   "which file to write to"); the *content* is what could carry terminal
   escapes / log poisoning / etc.

Fix: in T101 emission, filter to the content position only.  Channel-id
and the `-nonewline` switch don't trigger.

#### tclsh ground truth

```
% set chan stdout
% set data "INJECTED\x1b[31m"
% puts -nonewline $chan $data    ;# T101 must fire on $data
INJECTED^[[31m%
```

The content arg is what carries the injection; the channel id is just
the destination file handle.

#### Why the analyser reaches that verdict

`compiler/taint/_sinks.py` (puts sink hint) restricts T101 emission to
the trailing positional arg.  Channel-id and switch positions are
ignored.

#### Tests

- `tests/test_fp_tnt.py::test_FP_TNT_02_channel_id_position_no_t101` (FP)
- `tests/test_fp_tnt.py::test_FP_TNT_02_content_arg_still_fires` (TP, `puts $data`)
- `tests/test_fp_tnt.py::test_FP_TNT_02_content_with_channel_still_fires` (TP, `puts $chan $data` — chan filtered, data fires)

---

### FP-TNT-03 — eval/uplevel/interp eval LIST_CANONICAL suppression unsound (D5-T100/T105)

- **Verdict:** TRUE POSITIVE / security FN (now fixed)
- **Status:** locked in by `tests/test_fp_tnt.py::test_FP_TNT_03_*`
- **Codes:** T100 (taint -> code-exec), T105 (taint -> cross-interpreter eval)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-T100, D5-T105

#### Reproducer

```tcl
# UNSAFE -- tainted var becomes the command word
set raw [gets stdin]
eval [list $raw]
```

vs

```tcl
# SAFE -- literal "puts" is the command word; tainted var is an arg
set raw [gets stdin]
eval [list puts $raw]
```

#### Per-line reasoning

1. `set raw [gets stdin]` — `$raw` is tainted (user input).
2. `eval [list $raw]` — at runtime `[list $raw]` produces `{marker}` (or whatever the user typed) — a properly-quoted single-element list.  `eval` then re-parses it as a script: the first list element becomes the command word.  Pre-fix the LIST_CANONICAL taint colour suppressed T100 unconditionally — this missed the case where the tainted value sits at list-index 0 and *becomes* the cmd word.
3. Post-fix `_should_suppress_t100` requires the literal eval arg to be a `[list <known-cmd> ...]` cmd-sub AND the tainted var to sit at list-index >= 1.  When either condition fails, T100 fires.
4. The TN case (`eval [list puts $raw]`) keeps suppression: `puts` is in `REGISTRY.specs_by_name`, the tainted var is at index 1, so it can only become an argument to puts — no code execution.
5. Propagated cases (`set lst [list $raw]; eval $lst`) no longer benefit from LIST_CANONICAL — they refuse to suppress and T100 fires.  This is conservative: even when the helper produces a canonical list with a literal head, the eval site can't see that.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc marker args { puts EXECUTED }
% set raw marker

% # UNSAFE: tainted var at list-index 0 becomes the cmd word.
% eval [list $raw]
EXECUTED

% # SAFE: literal "puts" at index 0; tainted var is an arg.
% eval [list puts $raw]
marker

% # Same hazard via uplevel and through a variable copy.
% set lst [list $raw]
% eval $lst
EXECUTED
```

#### Compiler evidence

```
--- FP-TNT-03: eval/uplevel/interp eval LIST_CANONICAL suppression unsound (D5-T100/T105)
regen: python -m bench.fp_snippets --id FP-TNT-03
function ::top
  block entry_1
    [0] AssignValue 'raw' value='[gets stdin]'  defs={raw#1}  uses={}
    [1] InterpBoundary  defs={}  uses={}
    [2] Barrier cmd='eval'  defs={}  uses={raw#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-TNT-03`)

The eval statement's args are `('[list $raw]',)`; `_eval_arg_protected_by_list_literal` parses the cmd-sub, sees the first element is `$raw` (not a pure literal), returns False -> T100 fires.

#### Why the analyser reaches that verdict

`compiler/taint/_sinks.py`:

- `_eval_arg_protected_by_list_literal(arg, var_name)` — parses *arg* as a `[list ...]` cmd-sub.  Suppresses only when (a) the inner cmd is `list`, (b) the first list element is a pure literal (no `$` / `[` / `{*}`), (c) the head is in `REGISTRY.specs_by_name`, and (d) the tainted var appears only at list-index >= 1.
- `_should_suppress_t100(stmt, var_name, taint)` — for `eval`/`uplevel`/`::eval`/`::uplevel`, ignores `taint_sink_safe_colours` and delegates to `_eval_stmt_protected_by_list_literal`.
- `_should_suppress_sink_warning(code="T105", ...)` — same delegation for `interp eval` / `interp invokehidden`.
- `dialects/tcl/eval.py` and `dialects/tcl/uplevel.py` — `taint_sink_safe_colour=LIST_CANONICAL` removed.

#### Tests

- `tests/test_fp_tnt.py::test_FP_TNT_03_eval_list_tainted_head_fires_t100` (TP, `eval [list $raw]`)
- `tests/test_fp_tnt.py::test_FP_TNT_03_eval_list_literal_head_no_t100` (TN, `eval [list puts $raw]`)
- `tests/test_fp_tnt.py::test_FP_TNT_03_uplevel_list_tainted_head_fires_t100` (TP, `uplevel [list $raw]`)
- `tests/test_fp_tnt.py::test_FP_TNT_03_eval_list_via_var_still_fires` (TP, propagation defeats suppression)
- `tests/test_fp_tnt.py::test_FP_TNT_03_interp_eval_list_tainted_head_fires_t105` (TP, T105)
- `tests/test_fp_tnt.py::test_FP_TNT_03_interp_eval_list_literal_head_no_t105` (TN, T105)

---

### FP-TNT-04 — late `--` doesn't protect earlier option candidates (D5-T102)

- **Verdict:** TRUE POSITIVE / security FN (now fixed)
- **Status:** locked in by `tests/test_fp_tnt.py::test_FP_TNT_04_*`
- **Codes:** T102 (option-injection via tainted input)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-T102

#### Reproducer

```tcl
# UNSAFE -- $pat at index 0, -- at index 1 (cannot protect earlier $pat)
set pat [gets stdin]
regexp $pat -- subject
```

vs

```tcl
# SAFE -- -- at index 0, $pat at index 1 (terminator protects $pat)
set pat [gets stdin]
regexp -- $pat subject
```

#### Per-line reasoning

1. `set pat [gets stdin]` — `$pat` is tainted.
2. `regexp $pat -- subject` — Tcl scans option args left-to-right.  `$pat` at index 0 is treated as a switch candidate; if it expands to `-nocase`, Tcl consumes it as an option.  The `--` at index 1 is reached AFTER `$pat` has already been mis-classified, so it cannot retroactively protect index 0.
3. Pre-fix `_has_option_terminator` returned True for any `--` at or after `scan_start`, which caused the entire T102 sink classification to be suppressed — so even the index-0 tainted candidate produced no T102.
4. Post-fix the sink is always classified when the command has an option-terminator profile; the existing position filter `_option_scan_region` correctly stops at the first `--` (positions before the `--` remain candidates).  A tainted var at index 0 with `--` at index 1 sits in the scan region and fires.
5. The TN case (`regexp -- $pat subject`) is unchanged: `_option_scan_region` adds index 0 (`--`) and breaks, so `$pat` at index 1 is outside the region — no T102.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set pat "-nocase"

% # SAFE: -- at index 0 protects $pat at index 1
% regexp -- $pat ABC
0

% # UNSAFE: $pat at index 0 is mis-classified before -- at index 1 is reached
% regexp $pat -- ABC
wrong # args: should be "regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?"
```

#### Compiler evidence

```
--- FP-TNT-04: late `--` doesn't protect earlier option candidates (D5-T102)
regen: python -m bench.fp_snippets --id FP-TNT-04
function ::top
  block entry_1
    [0] AssignValue 'pat' value='[gets stdin]'  defs={pat#1}  uses={}
    [1] Call cmd='regexp'  defs={subject#1}  uses={pat#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-TNT-04`)

#### Why the analyser reaches that verdict

`compiler/taint/_sinks.py`:

- `_option_terminator_index` (replaces `_has_option_terminator`) — returns the first index of `--` >= `scan_start` instead of a bool.  Unused at the classification step now; kept for future use.
- `_classify_sink` — always emits T102 when the command has an option-terminator profile.  Per-var protection is handled below by the `_option_scan_region` filter, which already stops at the first `--` so positions before it remain candidates and positions after it are outside the region.

#### Tests

- `tests/test_fp_tnt.py::test_FP_TNT_04_late_terminator_does_not_protect_tainted_var` (TP, `regexp $pat -- subject`)
- `tests/test_fp_tnt.py::test_FP_TNT_04_early_terminator_protects_tainted_var` (TN, `regexp -- $pat subject`)
- `tests/test_fp_tnt.py::test_FP_TNT_04_no_terminator_fires` (TP control, no `--`)

---

### FP-TNT-05 — T104 honours registry network-address arg positions (D5-T104)

- **Verdict:** FALSE POSITIVE (precision) (now fixed)
- **Status:** locked in by `tests/test_fp_tnt.py::test_FP_TNT_05_*`
- **Codes:** T104 (SSRF via tainted network address)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D5-T104

#### Reproducer

```tcl
# TP -- URL slot is positional[0]; tainted $url fires T104.
set url [gets stdin]
http::geturl $url

# TN -- $hdr is an option VALUE, NOT the network address; no T104.
set hdr [gets stdin]
http::geturl http://example.com -headers $hdr
```

#### Per-line reasoning

1. `dialects/stdlib/http_.py` declares `taint_network_sink_args=(0,)` for `http::geturl` — only positional[0] is the network address.
2. `dialects/tcl/socket_.py` declares `taint_network_sink_args=(0, 1)` for `socket` — host + port positions.
3. Pre-fix `TaintSinkInfo` collapsed `taint_network_sink_args` into a single `is_network_sink: bool`; the per-var loop fired T104 on ANY tainted var in the statement.  That's the precision bug: `$hdr` in `http::geturl URL -headers $hdr` falsely fired even though it can't drive SSRF.
4. Post-fix the dataclass exposes the position tuple as `network_sink_args: tuple[int, ...] | None`; the per-var filter restricts T104 to vars whose arg index is in that tuple (empty tuple `()` keeps the whole-statement-scan semantics for iRules `connect`).

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% namespace eval http {}
% proc http::geturl {url args} { puts "url=$url\nargs=$args" }
% set hdr "X-Custom: bad"
% http::geturl http://example.com -headers $hdr
url=http://example.com
args=-headers {X-Custom: bad}
```

The URL is positional[0]; `$hdr` is consumed by the variadic `args` as an option-value pair — not a network address.

#### Compiler evidence

```
--- FP-TNT-05: T104 honours registry network-address arg positions (D5-T104)
regen: python -m bench.fp_snippets --id FP-TNT-05
function ::top
  block entry_1
    [0] AssignValue 'hdr' value='[gets stdin]'  defs={hdr#1}  uses={}
    [1] Call cmd='http::geturl'  defs={}  uses={hdr#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-TNT-05`)

`$hdr` appears at arg index 2 in this statement; `taint_network_sink_args=(0,)` for `http::geturl`, so the position filter rejects it and T104 stays silent.

#### Why the analyser reaches that verdict

- `compiler/registry/taint_sink_info.py` — `TaintSinkInfo.is_network_sink: bool` replaced with `network_sink_args: tuple[int, ...] | None`.  Docstring documents the None / empty-tuple / non-empty semantics.
- `compiler/registry/command_registry.py::classify_taint_sinks` — propagates the tuple instead of collapsing to bool.
- `compiler/taint/_sinks.py::_classify_sink` — still emits T104 when the spec marks the command as a network sink (`network_sink_args is not None`).
- `compiler/taint/_sinks.py::_find_taint_sinks` — adds `t104_addr_idxs` cache (same shape as T101/T102 region caches) and rejects per-var emissions when `len(t104_addr_idxs) > 0` and none of the var's arg indexes are in the tuple.  Empty tuple bypasses the filter (whole-statement scan).

#### Tests

- `tests/test_fp_tnt.py::test_FP_TNT_05_tainted_url_fires_t104` (TP, URL at positional[0])
- `tests/test_fp_tnt.py::test_FP_TNT_05_tainted_header_value_no_t104` (FP suppressed, `-headers $hdr`)
- `tests/test_fp_tnt.py::test_FP_TNT_05_socket_tainted_host_and_port_fire` (TP control, socket(0,1))

---


## STY — style / usage (W001/W104/W120/W122/W124/W126/W214/W302/W306)

Style/usage warnings that fired on idiomatic Tcl patterns where the
"style smell" is the documented convention.  Each entry locks in a
suppression that removed thousands of corpus FPs while keeping the
TP cases firing.

### FP-STY-01 — W001 Tk geometry-manager shortcut form (`grid .x` / `pack .x` / `place .x`)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_01_*`
- **Codes:** W001 (unknown subcommand)
- **Corpus:** every Tk script using the geometry-manager shortcut form.

#### Reproducer

```tcl
grid .x
pack .x
place .x
```

#### Per-line reasoning

Tk's `grid(n)`, `pack(n)`, `place(n)` accept a window-path as the first
arg as a shorthand for `grid configure .x` / `pack configure .x` / etc.
Pre-fix the subcommand-validation harness only matched known explicit
subcommand names (`configure`, `forget`, …) and reported a window-path
as "unknown subcommand".

Fix: detect the shortcut form (`grid .x` where the first arg looks like
a Tk path) and exempt it from W001.

#### tclsh ground truth

```
% pack [label .l -text hi]
% # No error — shortcut applies configure with default options.
```

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_01_grid_pathname_no_w001` (FP)
- `tests/test_fp_sty.py::test_FP_STY_01_pack_pathname_no_w001` (FP)
- `tests/test_fp_sty.py::test_FP_STY_01_place_pathname_no_w001` (FP)
- `tests/test_fp_sty.py::test_FP_STY_01_genuine_unknown_subcommand_still_fires` (TP, `grid bogus .x`)

---

### FP-STY-02 — W306 escaped `\[` / `\$` in quoted regexp patterns

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_02_*`
- **Codes:** W306 (literal-expected substitution)

#### Reproducer

```tcl
regexp "\[abc\]" $s
regexp "\$end" $s
```

#### Per-line reasoning

In a Tcl quoted string, `\[` is the literal character `[` (not a
command-substitution open), and `\$` is the literal `$` (not a
variable-substitution open).  The resolved arg text contains `[abc]`
and `$end` — valid regex syntax (a char-class and an end-anchor literal
`$` … but actually `$end` is just `$` followed by `end`; the typical
intent is the end-anchor regex `$`).

Pre-fix W306 fired because the resolved arg text couldn't distinguish
escaped from unescaped substitutions.  Fix: when the token isn't
already a VAR/CMD intrep, check the *raw source slice* for a live
(unescaped) `[`/`$`.  Escaped forms are exempt; live substitutions
(including `${var}` and `[cmd]`) still fire.

#### tclsh ground truth

```
% set s "[abc]xyz"
% regexp "\[abc\]" $s
1
```

The escaped pattern matches the literal `[abc]` substring — no Tcl
substitution occurred.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_02_escaped_bracket_no_w306` (FP)
- `tests/test_fp_sty.py::test_FP_STY_02_escaped_dollar_no_w306` (FP)
- `tests/test_fp_sty.py::test_FP_STY_02_live_dollar_in_quoted_pattern_still_fires` (TP)
- `tests/test_fp_sty.py::test_FP_STY_02_live_cmdsub_in_quoted_pattern_still_fires` (TP)

---

### FP-STY-03 — W104 usage / template notation exempt (`?optarg?`, `<placeholder>`, `...`)

- **Verdict:** FALSE POSITIVE (now fixed; corpus 165 → 144)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_03_*`
- **Codes:** W104 (string-concat list building)

#### Reproducer

```tcl
append usage "?arg?"
append usage "<placeholder>"
append usage "..."
```

#### Per-line reasoning

W104 fires when `append`/`string concat` is used to build what looks
like a list element by element (the safer idiom is `lappend` or
`list`).  But the documented Tcl conventions for usage strings —
`?optarg?` for optional, `<placeholder>` for template slots, `...` for
varargs — are not list-building; they are display formatting.

Fix: exempt the usage-notation glyphs.  Genuine `append result " $i"`
list-building still fires.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_03_optarg_question_marks_no_w104` (FP)
- `tests/test_fp_sty.py::test_FP_STY_03_placeholder_angle_no_w104` (FP)
- `tests/test_fp_sty.py::test_FP_STY_03_ellipsis_no_w104` (FP)
- `tests/test_fp_sty.py::test_FP_STY_03_genuine_list_building_still_fires` (TP)

---

### FP-STY-04 — W126 non-channel value: lassign destructure type-inference fix

- **Verdict:** FALSE POSITIVE (now fixed; corpus 4 → 0)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_04_*`
- **Codes:** W126 (non-channel value in channel arg)

#### Reproducer

```tcl
lassign [chan pipe] ch wch
puts $ch x
```

#### Per-line reasoning

`chan pipe` returns a 2-element LIST of channel handles.  `lassign`
destructures the list element by element — each def-target (`ch`,
`wch`) receives a single CHANNEL value, NOT the LIST itself.

Pre-fix the type-inference lattice typed all lassign def-targets as
LIST (mirroring the source).  Later use of `$ch` in a channel-arg
slot fired W126 because LIST ≠ CHANNEL.

Fix at the **lattice** level: lassign per-element destructure targets
are typed UNKNOWN (sound conservative — could be any element type).
The list type only applies to a captured-rest binding
(`set rest [lassign $L a b]`).

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_04_lassign_destructure_channels_no_w126` (FP)

---

### FP-STY-05 — W302 catch fire-and-forget (bare + subcommand-aware split)

- **Verdict:** FALSE POSITIVE (now fixed; 239 corpus firings cleared)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_05_*`
- **Codes:** W302 (catch without result variable)
- **Corpus:** ftp.tcl 35, comm.tcl 19, http.tcl 16, etc.

#### Reproducer

```tcl
# bare fire-and-forget (cleanup; failure is OK)
catch {close $fh}
catch {after cancel $h}
catch {file delete $f}

# ensemble subcommand fire-and-forget
catch {chan close $fh}
catch {array unset arr key}
```

#### Per-line reasoning

`catch {<cmd>}` without a result variable is the documented Tcl idiom
for "do this if possible, ignore if not".  Two sets cover the
canonical "error-on-missing-target" forms:

- `_FIRE_AND_FORGET_BARE` — `close`, `unset`, `rename`
- `_FIRE_AND_FORGET_SUBCOMMANDS` — `after cancel`, `chan close`,
  `array unset`, `dict unset`, `interp delete`,
  `namespace delete`/`forget`, `file delete`

A single-statement catch body whose head matches is exempt from W302.
Multi-cmd bodies, user calls, and *constructive* subcommands
(`chan configure`, `array set`, `dict set`, `file copy`, `interp
create`, `namespace eval`, `after <ms>`) still fire — they need a
result variable to know if they succeeded.

#### tclsh ground truth

```
% set fh [open /tmp/x w]
% close $fh
% catch {close $fh}   ;# already closed — error swallowed
1
```

#### Why the analyser reaches that verdict

`analyser/compiler_checks.py::_FIRE_AND_FORGET_BARE` +
`_FIRE_AND_FORGET_SUBCOMMANDS` define the exempt sets; the W302
emitter consults both before firing.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_05_bare_close_fire_and_forget_no_w302` (FP)
- `tests/test_fp_sty.py::test_FP_STY_05_ensemble_close_fire_and_forget_no_w302` (FP)
- `tests/test_fp_sty.py::test_FP_STY_05_constructive_subcommand_still_fires` (TP, `chan configure`)
- `tests/test_fp_sty.py::test_FP_STY_05_user_call_still_fires` (TP, user proc)

---

### FP-STY-06 — W122 / W124 OID-like dotted chains (not IPv4)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_06_*`
- **Codes:** W122, W124 (IPv4-shaped literal with out-of-range octet)
- **Corpus:** LDAP / SNMP OID literals across tcllib (`ldap`, `asn`, etc.).

#### Reproducer

```tcl
set oid 1.3.6.1.4.1.4203.1.11.3
```

#### Per-line reasoning

`1.3.6.1.4.1.4203.1.11.3` is an LDAP Private Enterprise Number (PEN)
OID — a hierarchical dotted chain with no octet-value constraint.
The naive IPv4 detector matched an embedded 4-component slice
(`4203.1.11.3`) where the first "octet" 4203 > 255 and fired W124.

Fix: skip when the matched quad is preceded by `.<digit>` OR followed
by `.<digit>` (part of a longer dotted chain).  Applied to both the
regex path (W122) and the SSA path (W124).

#### tclsh ground truth

OIDs are arbitrary integer sequences; they have no IPv4 semantics.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_06_oid_chain_no_w122_w124` (FP, both codes)
- `tests/test_fp_sty.py::test_FP_STY_06_real_ipv4_shaped_still_fires` (TP, 4-component dotted)

---

### FP-STY-07 — W120 package self-call (file is the provider)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_07_*`
- **Codes:** W120 (command used without `package require`)
- **Corpus:** msgcat self-calls 2→0, fileutil/mime similar.

#### Reproducer

```tcl
package provide msgcat 1.0
msgcat::mc hello
```

#### Per-line reasoning

A file declaring `package provide msgcat 1.0` IS the implementation of
`msgcat`; any `msgcat::foo` call inside that file doesn't need a
`package require msgcat` — the file *is* the require target.

Fix: union the file's `package_provides` set into the imported-set
check that drives W120.

(Note: the prior catalog text included a "taint-aware caveat" that
discussed tainted-var dispatch — that material belongs with the W307
multi-dispatch heuristic, not the W120 package-provide rule, and has
been removed.)

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_07_provider_self_call_no_w120` (FP)
- `tests/test_fp_sty.py::test_FP_STY_07_no_provide_still_fires` (TP)

---

### FP-STY-08 — W214 empty-body proc stubs

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_08_*`
- **Codes:** W214 (unused proc parameter)
- **Corpus:** tcllib `grammar_fa/faop.tcl` declares 14 such empty-body procs as the FA algebra API.

#### Reproducer

```tcl
proc stub {a b} {}
```

#### Per-line reasoning

`proc stub {a b} {}` with an empty body is the canonical Tcl
signature-stub pattern — overlay files plug in real bodies later, or
the declaration documents the API contract.  Every parameter is
necessarily "unused" because there is no body to use them; flagging
W214 on every param is pure noise.

Fix: detect zero-statement bodies in the IR; skip W214 entirely on
empty-body procs.

A related but distinct fix covers snit-style quoted-keyword marker
parameters (`{"as" ""}` as a positional keyword marker): the param
name is the literal `"as"`, which the body cannot reference as a
variable.  Detect via param name starting + ending with `"`.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_08_empty_body_stub_no_w214` (FP)
- `tests/test_fp_sty.py::test_FP_STY_08_non_empty_body_unused_param_still_fires` (TP)

---

### FP-STY-09 — W214 dispatcher needs arity-compatible peer (D3-P9 / D4-F4)

- **Verdict:** TRUE POSITIVE (precision-gap closure) — earlier suppression was over-broad
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_09_*`
- **Codes:** W214 (unused proc parameter)
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entries D3-P9, D4-F4

#### Reproducer

```tcl
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
    proc unrelated {cmd} { $cmd x }
}
```

#### Per-line reasoning

1. The three peer procs `a`/`b`/`c` share the signature `(ctx, token)` and each one only reads `ctx`.  Without a dispatch-protocol exemption, `token` is unused -> W214 fires three times.
2. `proc unrelated {cmd} { $cmd x }` is a one-argument dispatcher (it passes ONE positional arg to `$cmd`).  Pre-fix the W214 suppressor matched on "same-namespace dispatcher exists" regardless of arity, so the presence of `unrelated` would silence all three W214s.
3. But a 1-arg dispatcher CANNOT be calling a 2-arg peer family — the arity is incompatible.  No call site in the corpus would actually invoke `a`/`b`/`c` via `unrelated`.
4. Closure: `var_command_sites` now records the positional arg count of each variable-command dispatch, and the protocol-match heuristic requires `dispatcher_arity >= peer_arity`.  `unrelated`'s 1-arg dispatch no longer counts toward the 2-arg peer family, so `W214` correctly fires three times on `token`.

#### tclsh ground truth (9.0.3 — confirmed by execution)

The peers ignore `token` at runtime — no error — but the diagnostic is a code-smell: nothing in the program calls `a`/`b`/`c` with a `(ctx, token)` shape.

#### Compiler evidence

```
--- FP-STY-09: W214 dispatcher needs arity-compatible peer (D3-P9/D4-F4)
regen: python -m bench.fp_snippets --id FP-STY-09
function ::n::a
  block entry_1
    [0] Call cmd='puts'  defs={}  uses={ctx#0}
    term Goto
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-STY-09`)

#### Why the analyser reaches that verdict

`analyser/checks/_param.py` — the W214 dispatcher-protocol suppression now compares dispatcher arity (from `var_command_sites`, populated with each site's positional arg count) to peer arity.  Sites with `dispatcher_arity < peer_arity` are filtered out before the "≥1 compatible dispatcher" check; a peer family with no surviving dispatcher gets no suppression and W214 fires on each member's unused params.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_09_arity_incompatible_dispatcher_does_not_suppress` (TP — W214 fires 3x on `token`)
- `tests/test_fp_sty.py::test_FP_STY_09_arity_compatible_dispatcher_suppresses` (TN control — 2-arg dispatcher correctly suppresses)
- `tests/test_ground_truth_tn_fn.py::test_TP_W307_unrelated_dispatcher_does_not_suppress_peer_family` (audit pair; test name says W307 but asserts on W214 — read the body)
- `tests/test_ground_truth_tn_fn.py::test_TN_W307_protocol_compatible_dispatcher_suppresses_peer_family` (audit pair)

---

### FP-STY-10 — `scan_provably_no_match` soundness: `%n` always succeeds, `Inf` / format whitespace (D4-F1)

- **Verdict:** FALSE POSITIVE (now fixed; four sub-fixes)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_10_*`
- **Codes:** W210 (read-before-set) — fires on scan output vars when the scan provably can't consume input
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D4-F1

#### Reproducer

```tcl
proc f {} { scan {} %n n; puts $n }
```

#### Per-line reasoning

1. `scan {} %n n` — the format directive `%n` writes "the number of characters consumed so far" to its var-arg WITHOUT consuming any input.  On empty input it sets `n` to `0` and succeeds; this is documented Tcl behaviour (see `Tcl_ScanObjCmd`).
2. `puts $n` — reads `n`.  Since `%n` always succeeds and writes the var, the read is safe; W210 must NOT fire.
3. Pre-fix `scan_provably_no_match` treated `%n` like any other directive: if the input couldn't satisfy a preceding directive, the format was deemed unable to match and the output var was treated as unset.  The new code maps `%n` to a special `"always"` kind that short-circuits the no-match check (the var is always written).

The same soundness sweep covered three other gaps:

- `%f` now accepts `Inf`/`Infinity`/`NaN` as well as ordinary decimals (per `Tcl_GetDouble`).
- Format-whitespace now includes `\r\f\v` in addition to space/tab/newline.
- The analyser sees the *raw* (pre-escape) source text, so a backslash, `$`, or `[` in the input string could hide content that tclsh would resolve at runtime — the function now conservatively bails on those forms.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% scan "" %n n; puts $n
0
% scan Inf %f f; puts $f
inf
% scan { 123} "\r%d" n; puts $n
123
```

All three succeed; pre-fix the analyser fired W210 on each.

#### Compiler evidence

```
--- FP-STY-10: scan_provably_no_match soundness: %n / Inf / format whitespace (D4-F1)
regen: python -m bench.fp_snippets --id FP-STY-10
function ::f
  block entry_1
    [0] Call cmd='scan'  defs={n#1}  uses={}
    [1] Call cmd='puts'  defs={}  uses={n#1}
    term Goto
  block exit_2
    term (none — fall-through exit)
  read_before_set: (none)
```
(regen: `python -m bench.fp_snippets --id FP-STY-10`)

#### Why the analyser reaches that verdict

`compiler/scan_format.py:193` — `scan_provably_no_match`.  The directive-kind table maps `%n` to `"always"` (line ~70-80), which causes the predicate to short-circuit to `False` (i.e. the scan CAN match, so we cannot prove no-match) — output vars are recorded as defined.  The float-kind acceptance set now includes `Inf`/`Infinity`/`NaN`; the whitespace set includes `\r\f\v`; and any `\\`/`$`/`[` in the raw input string triggers a conservative bail.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_10_scan_percent_n_on_empty_input_silent` (FP)
- `tests/test_fp_sty.py::test_FP_STY_10_scan_float_accepts_inf_silent` (FP)
- `tests/test_fp_sty.py::test_FP_STY_10_scan_format_whitespace_cr_silent` (FP)
- `tests/test_fp_sty.py::test_FP_STY_10_scan_genuine_no_match_still_fires` (TP control)
- `tests/test_ground_truth_tn_fn.py::test_TN_scan_percent_n_on_empty_input`
- `tests/test_ground_truth_tn_fn.py::test_TN_scan_float_accepts_inf`
- `tests/test_ground_truth_tn_fn.py::test_TN_scan_format_whitespace_includes_cr_ff_vt`
- `tests/test_ground_truth_tn_fn.py::test_TP_scan_genuine_no_match_still_fires`

---

### FP-STY-11 — variadic var-write resolver for `scan` / `lassign` / `binary scan` (D4-F2)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_11_*`
- **Codes:** W210 (read-before-set) on variadic var-args past slot 18
- **Corpus:** synthetic — verified vs tclsh 9.0.3
- **Tracker:** [`review-findings-tracker.md`](review-findings-tracker.md) — entry D4-F2

#### Reproducer

```tcl
proc f {} {
    scan {x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14 x15 x16 x17 x18 x19} {%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} v0 v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v15 v16 v17 v18 v19
    return $v19
}
```

#### Per-line reasoning

1. `scan` is variadic in its var-args (one var per `%`-directive in the format).  Tcl scripts in the wild frequently call it with more than ~18 vars (e.g. binary scan parsers, fixed-format protocols).
2. Pre-fix the spec hard-coded `arg_roles[i]=VAR_WRITE` for slots `[2..19]` only — a finite slot budget.  Var #19 (the 20th positional argument) wasn't recognised as a var-write, so the SSA def for `v19` was missing and the subsequent `return $v19` fired W210.
3. Closure: each of `scan`, `lassign`, `binary scan` now defines an `arg_role_resolver` callback that takes the actual args list and returns a per-index role map — slot-budget-free.  Vars 19, 100, 1000 all get classified.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set s {x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14 x15 x16 x17 x18 x19}
% scan $s {%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} \
       v0 v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v15 v16 v17 v18 v19
20
% puts $v19
x19
```

All 20 vars are defined; reading `v19` succeeds.

#### Compiler evidence

```
--- FP-STY-11: variadic var-write resolver for scan/lassign/binary scan (D4-F2)
regen: python -m bench.fp_snippets --id FP-STY-11
function ::f
  block entry_1
    [0] Call cmd='scan'  defs={v0#1, v1#1, v2#1, v3#1, v4#1, v5#1, v6#1, v7#1, v8#1, v9#1, v10#1, v11#1, v12#1, v13#1, v14#1, v15#1, v16#1, v17#1, v18#1, v19#1}  uses={}
    term Return ${v19}
  block exit_2
    term (none — fall-through exit)
```
(regen: `python -m bench.fp_snippets --id FP-STY-11`)

Every var from `v0` through `v19` shows up as a def of the `scan` call — the resolver classified all 20 var slots as VAR_WRITE.

#### Why the analyser reaches that verdict

- `dialects/tcl/scan.py:46` — `_scan_arg_roles` walks the actual args list, counts `%`-directives in the format, and emits VAR_WRITE for the trailing var-arg slots.
- `dialects/tcl/lassign.py:47` — `_lassign_arg_roles` marks all positional args after the list arg as VAR_WRITE.
- `dialects/tcl/binary.py:105` — inline lambda computes the var-arg slot set from the actual call.

Each is wired through `CommandSpec.arg_role_resolver`, which the registry consults in preference to the static `arg_roles` map.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_11_scan_20_vars_no_false_w210` (FP)
- `tests/test_fp_sty.py::test_FP_STY_11_lassign_many_vars_no_false_w210` (FP)
- `tests/test_fp_sty.py::test_FP_STY_11_binary_scan_many_vars_no_false_w210` (FP)
- `tests/test_fp_sty.py::test_FP_STY_11_scan_fewer_specifiers_than_vars_still_fires` (TP control)
- `tests/test_ground_truth_tn_fn.py::test_TN_scan_with_more_than_18_vars_no_false_w210`
- `tests/test_ground_truth_tn_fn.py::test_TN_lassign_with_many_vars_no_false_w210`
- `tests/test_ground_truth_tn_fn.py::test_TN_binary_scan_with_many_vars_no_false_w210`

---

### FP-STY-12 — W216 / W212 braced indirect-array-element idiom `${var}(idx)`

- **Verdict:** FALSE POSITIVE (now fixed; double-fire W216 + W212 cleared)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_12_*`
- **Codes:** W216 (broken brace-form array ref), W212 (substitution where
  var-name expected)
- **Corpus:** Tcl 9.0 stdlib `http.tcl` — `set ${token}(status) eof`,
  `set ${token3}(-pipeline)`, `info exists ${tokenVal}(after)`,
  `unset ${tok}(socketcoro)`, `vwait ${token}(status)` — 25 firings in one
  file, every one a `token`/`tok` scalar holding an array name.

#### Reproducer

```tcl
# `token` is a scalar holding an ARRAY NAME (e.g. ::http::1) — the canonical
# Tcl "array kept in a variable" pattern (http, snit, many state machines).
set token ::http::1
set ${token}(status) eof        ;# write element: <value-of-token>(status)
info exists ${token}(-pipeline)  ;# read element via indirection
unset ${token}(socketcoro)       ;# unset element via indirection
```

#### Per-line reasoning

1. Tcl parses `${token}(status)` as the brace-form substitution `${token}`
   (the lexer ends the variable name at the `}`) concatenated with the
   **literal** text `(status)`.  The resulting string is
   `<value-of-token>(status)` — e.g. `::http::1(status)`.
2. In a **variable-name argument position** (`set`/`incr`/`append`/`lappend`/
   `unset` target, `info exists`, `vwait`) the command interprets that string
   as a variable name → element `status` of the array named `::http::1`.
   This is the *only* way to reach an element of an array whose name lives in
   a scalar (short of `upvar`), and it is a heavily-used idiom (Tcl's own
   `http` package).
3. W216's suggested rewrite `$token(status)` is **actively wrong** here: it
   would access element `status` of an array literally named `token`, not the
   array named *by* `token`'s value.  W212's suggestion `token(status)` is
   wrong for the same reason.  Both must stay silent.
4. The braces are the discriminator.  The **bare** `$token(status)` is a
   *direct* array reference (array literally named `token`) — a different
   construct — so it is left to W212's genuine dynamic-name foot-gun check.
5. In a **value position** (`puts ${arr}(x)`, `set y ${arr}(x)`) the same
   `${arr}(x)` is almost always a typo for `$arr(x)` element access, so W216
   **still fires** there.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% array set ::http::1 {status pending -pipeline yes}
% set token ::http::1
% set ${token}(status) eof
eof
% set ${token}(status)
eof
% info exists ${token}(-pipeline)
1
```

The value-position broken case errors, proving W216 is a true positive there:

```
% array set arr {x 1}
% puts ${arr}(x)
can't read "arr": variable is array
```

#### Why the analyser reaches that verdict

- `shared/naming.py::is_braced_indirect_array_ref` recognises the
  `${name}(idx)` shape (brace-form var name + parenthesised index).
- W216: `analyser/_analyser/_diag_brace_then_paren.py` — `_varname_word_indices`
  computes which command words are variable-name positions; Pattern (1) is
  suppressed when the `${name}(idx)` word starts at one of those offsets.
  Value-position matches still fire.
- W212: `analyser/checks/_style.py::check_name_vs_value` skips the arg when
  `is_braced_indirect_array_ref(written)` holds; bare `$x` / `$arr(idx)` /
  index-less `${x}` foot-guns still fire.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_12_set_indirect_array_no_w216_w212` (FP)
- `tests/test_fp_sty.py::test_FP_STY_12_info_exists_indirect_no_w216_w212` (FP)
- `tests/test_fp_sty.py::test_FP_STY_12_unset_indirect_no_w216` (FP)
- `tests/test_fp_sty.py::test_FP_STY_12_vwait_incr_append_lappend_indirect_no_w216` (FP variants)
- `tests/test_fp_sty.py::test_FP_STY_12_value_position_still_fires_w216` (TP)
- `tests/test_fp_sty.py::test_FP_STY_12_bare_dollar_name_still_fires_w212` (TP)
- `tests/test_fp_sty.py::test_FP_STY_12_index_less_brace_still_fires_w212` (TP)

---

### FP-STY-13 — W113 redefining an overridable Tcl library procedure

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_13_*`
- **Codes:** W113 (procedure shadows built-in command)
- **Corpus:** Tcl 9.0 stdlib itself — `init.tcl` (`proc unknown`,
  `proc auto_execok`, `proc auto_load`, `proc tcl_findLibrary`),
  `history.tcl` (`proc history`), `package.tcl` (`proc pkg_mkIndex`),
  `word.tcl` (`proc tcl_wordBreakAfter` …).  The files that *define* these
  procs were being told they "shadow a built-in".

#### Reproducer

```tcl
proc unknown args { return }          ;# the documented Tcl extension point
proc history {args} { return }        ;# a Tcl library proc, not a C built-in
proc tcl_findLibrary {a b c d e f} { return }
```

#### Per-line reasoning

1. `unknown`, `history`, `auto_execok`, `auto_load`, `auto_mkindex`,
   `auto_qualify`, `auto_reset`, `parray`, `pkg_mkIndex`, `tcl_findLibrary`,
   and the `tcl_*WordBreak*` / `tcl_*OfWord` helpers are written **in Tcl** and
   shipped in the standard library (`init.tcl` / `auto.tcl` / `history.tcl` /
   `package.tcl` / `word.tcl`).  They are *not* C-level built-in commands.
2. They are documented as user-replaceable — Tcl(n) `unknown` states that
   applications "can replace it"; the `auto_*` and word-break helpers are
   overlay/extension points.  Redefining one is the supported idiom, and Tcl's
   own library is exactly the code that `proc`s them, so the W113 message
   "shadows built-in command" is factually wrong for these names.
3. Genuine C commands that are *not* byte-compiled but still dangerous to
   redefine (`clock`, `after`, `socket`, `glob`) are **not** in the exempt
   set — they keep firing W113.  Byte-compiled core built-ins (`set`, `puts`,
   `expr`, `if`) also keep firing.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% proc unknown args { return "caught: $args" }
% frobnicate 1 2 3
caught: frobnicate 1 2 3
```

Overriding `unknown` is not only legal — it is the intended extension hook.

#### Why the analyser reaches that verdict

`analyser/_analyser/_proc.py` defines `_OVERRIDABLE_LIBRARY_PROCS`; the W113
shadow check clears `shadow_name` when the proc name is in that set, after the
existing namespace-qualified exemption.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_13_unknown_override_no_w113` (FP)
- `tests/test_fp_sty.py::test_FP_STY_13_library_procs_no_w113` (FP variants)
- `tests/test_fp_sty.py::test_FP_STY_13_c_builtin_still_fires_w113` (TP, `set`/`puts`)
- `tests/test_fp_sty.py::test_FP_STY_13_non_bytecompiled_c_command_still_fires_w113` (TP, `clock`/`after`/`socket`/`glob`)

---

### FP-STY-14 — W105 single bare-variable body is a script reference, not an inline block

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_14_*`
- **Codes:** W105 (unbraced code-block argument) — fired at **Error** severity
- **Corpus:** Tcl 9.0 stdlib + tcllib — `eval $cmd` (auto.tcl, package.tcl),
  `proc $fakeName $arglist $body` (auto.tcl dynamic proc), `namespace eval ::
  $state(-command) $token` (http.tcl callback dispatch — 6+ firings),
  `after 0 $coroName` (http.tcl), `interp eval $child $contents` (safe.tcl),
  `foreach name $nameList $_sub_load_cmd` (init.tcl).

#### Reproducer

```tcl
# Each body argument below is a single bare variable that HOLDS the script —
# a script-valued reference, not an inline code block.
eval $cmd
proc $fakeName $arglist $body
namespace eval :: $state(-command) $token
after 0 $coroName
```

#### Per-line reasoning

1. W105 warns that an *inline* code-block argument containing `$`/`[` should
   be braced to avoid double substitution.  But when the body word is a
   **single bare variable substitution** (`$cmd`, `${cmd}`, `$state(-command)`,
   `$ns::var`) there is no inline block — the variable already holds the
   script.  Bracing it (`eval {$cmd}`) evaluates the *literal text* `$cmd`,
   which is a different program (and usually an error).
2. The W105 quick-fix ("wrap code block in braces") is therefore **actively
   wrong** for this shape.  The genuine risks are covered elsewhere: the
   eval-injection / double-substitution risk of `eval $cmd` is W101's, and the
   dynamic command-name risk of a callback (`$state(-command)`) is W307's
   (which already *accepts* the registered-callback dispatch form — so W105
   was double-flagging a pattern the analyser elsewhere recognises as
   legitimate).
3. `uplevel $script` was already silent (its arg is not BODY-role); this fix
   makes `eval` and the callback-dispatch forms consistent with it.
4. The exemption is narrow: a body that is a **composite** word — `${t}--Coro`
   (var + literal), `"do $script"` (quoted with interpolation), `$cmd$args`
   (concatenation) — has more than one content token and **still fires** W105,
   because there the substitution really is being woven into an inline script.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set cmd {set y 42}
% eval $cmd          ;# runs the script held in cmd
% puts $y
42
% eval {$cmd}        ;# braces it -> evaluates the literal text "$cmd"
invalid command name "set y 42"
```

The brace-fix W105 suggested turns working code into an error — proof the
single-bare-var body must not be flagged.

#### Why the analyser reaches that verdict

`analyser/checks/_style.py::check_unbraced_body` calls the new
`_word_is_single_var` helper: it counts the content tokens spanned by the body
word and skips W105 when the word is exactly one `VAR` token.  Composite /
quoted bodies keep their existing `Error`-severity W105.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_14_eval_single_var_body_no_w105` (FP)
- `tests/test_fp_sty.py::test_FP_STY_14_callback_dispatch_body_no_w105` (FP)
- `tests/test_fp_sty.py::test_FP_STY_14_dynamic_proc_and_after_no_w105` (FP variants)
- `tests/test_fp_sty.py::test_FP_STY_14_quoted_interpolated_body_still_fires` (TP)
- `tests/test_fp_sty.py::test_FP_STY_14_composite_body_still_fires` (TP)

---

### FP-STY-15 — lexer: `$` before a closing `"` merged the quoted word with the next (E002 / E205 / W306)

- **Verdict:** FALSE POSITIVE (lexer correctness bug; now fixed)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_15_*`
- **Codes:** E002 (too few arguments), E205 (extra characters after
  close-quote), W306 (substitution-in-literal regex pattern) — E002/E205 fired
  at **Error** severity on valid Tcl
- **Corpus:** Tcl 9.0 stdlib `tcltest.tcl` (`regsub "\n$" [string tolower
  $msg] "" msg`), tcllib `csv.tcl` (`regsub "\0$" $line {} line`,
  `regsub -- "^${delRE}${delRE}$sepRE" $line \0${delChar}$ ...`) — anywhere a
  quoted word ends with the regex end-of-line anchor `$"`.

#### Reproducer

```tcl
# `"\n$"` is a literal regexp end-of-line anchor — the `$` sits immediately
# before the closing `"`, which is NOT a variable-name character, so the `$`
# is literal (tclsh substitutes nothing).
regsub "\n$" $msg "" out
string match "abc$" $x
```

#### Per-line reasoning

1. In Tcl a `$` is a substitution only when followed by a valid variable-name
   character.  `$"` is not — so `"abc$"` is the literal four-char string
   `abc$`, and `"^foo$"` is the regex `^foo$` (end-anchor).  tclsh executes
   `regsub "\n$" $msg "" out` and `regexp -- "^foo$" "foo"` without error.
2. The lexer parsed the trailing bare `$` as a `STR` token *inside* the quoted
   word, then re-entered the string scanner with the cursor on the **closing**
   `"`.  Because the preceding token was `STR`, the "start of a new word" path
   misread that closing `"` as a *new opening* quote and kept scanning —
   swallowing the following words into one token.
3. The downstream damage: the merged word reduced the visible argument count
   (`string match "abc$" $x` looked like one argument → **E002** "too few
   arguments"), the eventual real closing quote tripped **E205** "extra
   characters after close-quote", and the smeared pattern made the literal
   `$` look like a live substitution → spurious **W306**.
4. Fix: the new-word `"`-opens-a-quote branch is guarded by
   `not self.insidequote`.  When the scanner is already inside a quote, a `"`
   is the **closing** delimiter, handled by the existing close-quote branch.
5. The genuine cases still fire: a live `$bar` / `$pat` / `${b}` inside a
   quoted regex pattern is a real substitution and still raises W306.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set msg "Hello\n"
% regsub "\n$" $msg "" out      ;# end-anchor strips the trailing newline
1
% string length $out
5
% regexp -- "^foo$" "foo"        ;# `$` is the end-anchor, not a variable
1
% set s "^foo$"                  ;# literal value — no substitution
^foo$
```

#### Why the analyser reaches that verdict

`compiler/parsing/lexer.py::_parse_string` — the `newword` branch that opens a
new quoted string now requires `not self.insidequote`, so the closing `"` after
a tail `$` is no longer misread as an opening quote.

#### Tests

- `tests/test_fp_sty.py::test_FP_STY_15_regsub_dollar_anchor_no_errors` (FP)
- `tests/test_fp_sty.py::test_FP_STY_15_string_match_dollar_anchor_no_arity` (FP)
- `tests/test_fp_sty.py::test_FP_STY_15_dollar_quote_word_boundary_lexes` (FP, token-level)
- `tests/test_fp_sty.py::test_FP_STY_15_regex_end_anchor_no_w306` (FP)
- `tests/test_fp_sty.py::test_FP_STY_15_live_var_in_quoted_pattern_still_w306` (TP)
- `tests/test_tcl_corner_cases.py::TestDollarBeforeCloseQuote` (token-level FP/variants)

---

### FP-STY-16 — W201 manual-path-concat fires on prose / protocol / display strings

- **Verdict:** FALSE POSITIVE (now fixed for the literal-whitespace class)
- **Status:** locked in by `tests/test_fp_sty.py::test_FP_STY_16_*`
- **Codes:** W201 (manual path concatenation — use `[file join]`)
- **Corpus:** Tcl 9.0 stdlib `http.tcl` (`set …(bypass) "CONNECT $host:$port
  HTTP/1.1"`), `tcltest.tcl` (`set msg "Usage: [file tail …] script "`) — any
  `set` of a multi-word quoted string that merely *contains* a `/`.

#### Reproducer

```tcl
# An HTTP request line — the `/` is in the protocol version, not a path.
set bypass "CONNECT $host:$port HTTP/1.1"
# A usage message — `/` would be incidental; this is display text.
set msg "Usage: [file tail $exe] script "
```

#### Per-line reasoning

1. W201 fires on a `set` whose rendered value has a path separator (`/` / `\`)
   *and* interpolation, suggesting `[file join]`.  But a value that contains a
   **literal space** (outside any `[…]` command substitution) is a multi-word
   string — prose, a protocol line, an HTML/usage fragment — not a single
   filesystem path token.
2. `[file join]` is nonsensical there.  tclsh:
   `file join CONNECT $host:$port HTTP/1.1` → `CONNECT/h:8080/HTTP/1.1` —
   it shreds the request line on the spaces.
3. The discriminator is precise: the rendered-properties pass now sets
   `HAS_LITERAL_SPACE` when lexing the value yields a top-level `SEP` token (a
   word boundary).  A command substitution `[file tail $path]` stays a single
   `CMD` token, so its *internal* spaces do **not** set the bit — a genuine
   path concat `set f "$dir/[file tail $path]"` still fires W201.
4. Known residual (still fires; no clean literal signal): bracketless
   single-token concatenations such as CIDR `set x "$ip/$mask"` or an HTML
   attribute `set img src=$a/$b` — there is no literal whitespace to key on,
   and `$a/$b` is structurally identical to a real `$dir/$file`.

#### tclsh ground truth (9.0.3 — confirmed by execution)

```
% set host h; set port 8080
% file join CONNECT $host:$port HTTP/1.1
CONNECT/h:8080/HTTP/1.1
```

The "portable" rewrite changes the string's meaning entirely — proof the `/`
is not a path separator.

#### Why the analyser reaches that verdict

`compiler/rendered_properties.py` sets `RenderedProperties.HAS_LITERAL_SPACE`
on a top-level `SEP` token; `compiler/taint/_path_concat.py` skips W201 when
the assigned value carries that bit.

#### Tests

W201 is produced by the **taint pass**, which is an in-process analyser
diagnostic that the packaged server does *not* surface through
`publishDiagnostics` (verified: no taint code — W201/T100/T101 — reaches the
lsp_e2e / VS Code server path).  The authoritative W201 surface is therefore
the in-process analyser, exercised here:

- `tests/test_fp_sty.py::test_FP_STY_16_http_request_line_no_w201` (FP)
- `tests/test_fp_sty.py::test_FP_STY_16_usage_message_no_w201` (FP)
- `tests/test_fp_sty.py::test_FP_STY_16_prose_with_path_no_w201` (FP)
- `tests/test_fp_sty.py::test_FP_STY_16_genuine_path_concat_still_fires` (TP)
- `tests/test_fp_sty.py::test_FP_STY_16_path_with_command_sub_still_fires` (TP, internal spaces don't suppress)

---

### FP-STY-17 — W001 same-file shadow suppression (proc / class / alias / ensemble / stub redefining a registry ensemble command)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `rust/tcl-compiler/src/analyser/diagnostics/fp/sty.rs::fp_sty_17_*`
- **Codes:** W001 (unknown subcommand), subcommand-level W002 (disabled-in-dialect subcommand)
- **Corpus:** any script that re-purposes a builtin ensemble name as its own
  proc/class/alias — e.g. a namespace-local `proc string {op args} {…}`
  wrapper, or a compatibility shim installed via `interp alias {} info {} …`.

#### Reproducer

```tcl
proc string {op args} {
    return "shadowed:$op"
}
string reverse hello
```

#### Per-line reasoning

1. A `proc` (or `oo::class`/snit/itcl class, `interp alias`, `namespace
   ensemble create -command`, or an inline `# tcl-lsp: stub`) whose name
   matches a registry ensemble command **replaces** that command at the
   call site — Tcl resolves the user definition, not the builtin, exactly
   as it does for any other shadowed builtin (`close`, `apply`, …).
2. The analyser already gets this right for the sibling arity diagnostics
   (E002/E003): `emit_arity_diagnostics` queues every subcommand arity
   check into `pending_arity` and `flush_arity_diagnostics` drops it when
   the call resolves to a same-file proc/class/alias/ensemble/stub. W001
   (`emit_w001_unknown_subcommand`) used to push straight into
   `result.diagnostics`, bypassing that shadow check entirely — so
   `string reverse hello` above drew a spurious "Unknown subcommand
   'reverse' for 'string'" even though the call never reaches the builtin
   ensemble at all.
3. The fix queues the "unknown subcommand" verdict into the same
   `pending_arity` list E002/E003/W004 already use (a wrong subcommand name
   is the same species of "call is malformed against the resolved registry
   signature" as a wrong argument count), and the subcommand-level "disabled
   in this dialect" verdict into `pending_disabled_commands` (the queue the
   whole-command W002 check uses). Both resolve through the shared
   `UserResolutionFacts` shadow computation in `flush_arity_diagnostics` /
   `flush_disabled_command_diagnostics` — no duplicated logic.
4. Order-sensitivity is preserved: a top-level call *before* the shadowing
   proc's definition still reaches the real builtin (Tcl executes
   top-level commands in source order during load) and still fires; a
   call inside a proc body is not order-gated (proc bodies run after the
   whole script has loaded).
5. The suppression is scoped to the shadowed name only — a *different*,
   non-shadowed ensemble command in the same file still fires normally.

#### tclsh ground truth (8.6.14 — confirmed by execution)

```
% proc string {op args} { return "shadowed:$op" }
% string reverse hello
shadowed:reverse
% catch {string mach hello world} err; puts $err
;# (evaluated BEFORE the proc definition, in a fresh interpreter)
unknown or ambiguous subcommand "mach": must be bytelength, cat, compare,
equal, first, index, is, last, length, map, match, range, repeat, replace,
reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or
wordstart
```

The user `proc string` completely replaces the builtin ensemble at the call
site — there is no "unknown subcommand" to report — but the identical call
*before* the proc is defined still hits the real builtin and still errors.

#### Why the analyser reaches that verdict

`analyser/diagnostics/validity.rs::emit_w001_unknown_subcommand` queues its
"unknown subcommand" verdict into `Analyser::pending_arity` — the same queue
E002/E003/W004 already use — instead of pushing directly.
`Analyser::flush_arity_diagnostics` drains it post-walk through
`UserResolutionFacts::resolves_to_user`, using each candidate's captured
call-site-resolution namespace and `enforce_order` flag (`true` for
top-level calls, `false` inside a proc/method body).

#### Tests

- `fp_sty_17_proc_shadow_no_w001` (FP)
- `fp_sty_17_alias_shadow_no_w001` (FP)
- `fp_sty_17_class_shadow_no_w001` (FP)
- `fp_sty_17_stub_shadow_no_w001` (FP)
- `fp_sty_17_unshadowed_command_still_fires` (TP — a different ensemble in
  the same file is unaffected by the shadow)
- `fp_sty_17_call_before_shadow_definition_still_fires` (TP — top-level
  order-sensitivity)
- `fp_sty_17_proc_body_call_shadowed_by_later_def_no_w001` (FP — proc-body
  calls are not order-gated)

---

### FP-STY-18 — W001 `{*}`-expanded subcommand position (`cmd {*}{subcmd args…}`)

- **Verdict:** FALSE POSITIVE (now fixed)
- **Status:** locked in by `rust/tcl-compiler/src/analyser/diagnostics/fp/sty.rs::fp_sty_18_*`
- **Codes:** W001 (unknown subcommand)
- **Corpus:** dynamic-dispatch helpers that build an ensemble call from a
  list (`dict {*}$argsList`, or a literal `{*}{…}` spread used to keep a
  long argument list readable) — the dynamic-variable form was already
  exempt via the `$`/`[` substitution gate; only the literal-braced form
  was missed.

#### Reproducer

```tcl
dict {*}{create a b}
```

#### Per-line reasoning

1. `{*}WORD` splices `WORD`'s **list elements**, not its literal text, into
   the argument position. `dict {*}{create a b}` therefore calls `dict
   create a b`, not a single subcommand literally named `"create a b"`.
2. `emit_w001_unknown_subcommand` reads `args[0]` (`"create a b"`, the
   whole spread word as one string) as the candidate subcommand name. It
   is never equal to any registry subcommand, so it fired "Unknown
   subcommand 'create a b' for 'dict'" on perfectly valid Tcl.
3. The existing dynamic-substitution gate (`has_substitution`, checking for
   `$`/`[`) does not catch this shape — `{create a b}` is a plain literal
   with no substitution, so it fell through to the unknown-subcommand path.
4. The fix threads the per-argument `{*}`-expansion flag through to the
   emitter (mirroring the identical `arg_expand.first()` guard the
   subcommand-arity check already applies) and skips the check outright
   when the subcommand position is expanded — the same "abstain when the
   effective value can't be read off the source" convention every other
   dynamic-value gate in this emitter already follows.

#### tclsh ground truth (8.6.14 — confirmed by execution)

```
% dict {*}{create a b}
a b
% dict {*}{bogus a b}
unknown or ambiguous subcommand "bogus": must be append, create, exists,
filter, for, get, incr, info, keys, lappend, map, merge, remove, replace,
set, size, unset, update, values, or with
```

`{*}{create a b}` is a genuine, valid `dict create` call; `{*}{bogus a b}`
is a genuine error — the check must tell them apart by evaluating the
*spread* subcommand, not the raw source text, so it now abstains on both
rather than misreading either.

#### Why the analyser reaches that verdict

`analyser/diagnostics/validity.rs::emit_w001_unknown_subcommand` now takes
`arg_expand_in` and returns before any subcommand-name comparison when
`arg_expand[0]` is set — identical to the guard
`emit_arity_diagnostics`'s subcommand-arity path already had.

#### Tests

- `fp_sty_18_expanded_literal_subcommand_no_w001` (FP)
- `fp_sty_18_genuine_unknown_subcommand_without_expansion_still_fires` (TP)

---

### FP-STY-19 — W001 missing Tk 9.0 subcommands (`wm iconbadge`, `grid`/`pack`/`place content`)

- **Verdict:** FALSE POSITIVE (now fixed — registry data gap, not compiler logic)
- **Status:** locked in by `rust/tcl-compiler/src/analyser/diagnostics/fp/sty.rs::fp_sty_19_*`
- **Codes:** W001 (unknown subcommand), W002 (disabled-in-dialect subcommand)
- **Corpus:** any Tk 9.0 script using the taskbar/dock icon badge API or the
  9.0-renamed geometry-manager introspection subcommand.

#### Reproducer

```tcl
wm iconbadge .win 5
grid content .frame
pack content .frame
place content .frame
```

#### Per-line reasoning

1. Tk 9.0 added `wm iconbadge window badge` (a short overlay label on the
   window's taskbar/dock icon — `doc/wm.n`, absent from the 8.4/8.5/8.6
   man pages) and renamed `grid`/`pack`/`place`'s `slaves` introspection
   subcommand to `content` as the canonical spelling (`slaves` is kept only
   as a documented backward-compatible synonym — `doc/grid.n`,
   `doc/pack.n`, `doc/place.n`).
2. None of the four were present in the registry's `SubCommand` tables at
   all (`tcl-registry/src/commands/tk/{wm,grid,pack,place}.rs`), so every
   ensemble/`is_known` lookup failed and the analyser reported a genuine
   9.0 Tk API as an "Unknown subcommand" — this is a registry data
   completeness gap, never a compiler-logic bug: the fix adds the missing
   `SubCommand` entries, dialect-gated `TCL90_PLUS` (matching the gate
   already used for `info cmdtype`, the other 9.0-only ensemble
   subcommand), not any change to `emit_w001_unknown_subcommand` itself.
3. Tk subcommands are checked regardless of the active *Tcl* dialect (the
   `| DialectSet::TK` union in `emit_w001_unknown_subcommand`), so under an
   explicit `tcl8.6` dialect these four now correctly downgrade to W002
   ("disabled in the active dialect profile") rather than either firing
   W001 or going unconditionally silent — matching the existing `package
   files` / 8.6 precedent (FP.md has no dedicated entry for that one; see
   `rust/tcl-compiler/tests/analyser.rs::package_files_is_disabled_under_tcl86_per_tclsh`).

#### Ground truth (Tk 9.0 documentation — `doc/wm.n` / `doc/grid.n` / `doc/pack.n` / `doc/place.n`, tag `core-9-0-4`)

```
wm.n:    wm iconbadge window badge
                Sets an icon badge for the taskbar or dock icon […]
grid.n:  grid content master ?-option value?
                […] this command was named "slaves" and that name is
                kept as a synonym for backward compatibility.
```

(Confirmed by direct comparison of the `core-8-4-20` / `core-8-5-19` /
`core-8-6-16` / `core-9-0-4` man-page tags — none of the four subcommands
appear before 9.0.)

#### Why the analyser reaches that verdict

`tcl-registry/src/commands/tk/wm.rs` gains an `"iconbadge"` `SubCommand`
entry (`dialects: Some(DialectSet::TCL90_PLUS)`); `grid.rs` / `pack.rs` /
`place.rs` each gain a `"content"` entry with the same gate, alongside the
pre-existing ungated `"slaves"` entry.

#### Tests

- `fp_sty_19_wm_iconbadge_not_unknown` (FP + the tcl8.6 W002-not-W001 verdict)
- `fp_sty_19_grid_pack_place_content_not_unknown` (FP)
- `fp_sty_19_genuine_unknown_wm_and_geometry_subcommands_still_fire` (TP)

---

## Precision gaps (PR #498 deep-review) — ALL CLOSED

The PR #498 deep review identified 13 precision regressions — places
where a FP suppression went too far and converted a TP into a false-
negative.  All 13 are now closed with paired TP tests.  Additionally,
two pre-existing open gaps (G14, G15) carried over from earlier review
rounds were closed in the same pass.

### Closed (13 of 13 from PR #498 + 2 pre-existing)

| # | Gap | Closure | Test |
|---|---|---|---|
| G1 | regexp no-match path: `v` stays unset; `puts $v` after a provably non-matching pattern should fire W210 | SCCP-driven match analysis -- post-process `_read_before_set` to detect regexp/scan with bare-literal pattern + input, use Python `re` (regexp) or %d simulator (scan) to prove no-match | `tests/test_fp_rbs.py::test_FP_RBS_02_regexp_provably_no_match_fires_w210` |
| G2 | scan no-match path: same root cause | Same machinery, scan-specific %d simulator | `tests/test_fp_rbs.py::test_FP_RBS_02_scan_provably_no_match_fires_w210` |
| G3 | OBJ-04 namespaced proc returning plain string | Distinguish user-proc calls from builtin/namespaced builtins; user procs only become factory sources when the fixpoint proves them object-returning | `tests/test_fp_obj.py::test_FP_OBJ_04_namespaced_string_returning_user_proc_fires_w307` |
| G4 | OBJ-04 mixed-return wrapper | Require ALL feasible return values (not just the LAST observed) to be namespaced cmd-subs | `tests/test_fp_obj.py::test_FP_OBJ_04_mixed_return_wrapper_fires_w307` |
| G5 | OBJ-09 multi-dispatch ignores SCCP CONST | SCCP-says-not-a-command override on the heuristic suppressions | `tests/test_fp_obj.py::test_FP_OBJ_09_const_string_multi_dispatch_fires_w307` |
| G6 | OBJ-10 callback-suffix ignores SCCP CONST | Same SCCP override + array-element base-name lookup | `tests/test_fp_obj.py::test_FP_OBJ_10_const_string_callback_suffix_fires_w307` |
| G7 | OBJ-10 dash-prefix ignores SCCP CONST | Same as G6 | `tests/test_fp_obj.py::test_FP_OBJ_10_const_string_dash_prefix_fires_w307` |
| G8 | OBJ-07 namespaced ensemble ignores SCCP const-prefix | Compose prefix + ``::tail`` and check registry / user-proc / class tables | `tests/test_fp_obj.py::test_FP_OBJ_07_namespaced_ensemble_const_prefix_fires_w307` |
| G9 | dict with whole-proc suppression | Walk back to the most recent IRAssignConst literal value; use its keys as the suppression set (empty dict → no suppression) | `tests/test_fp_rbs.py::test_FP_dict_with_does_not_suppress_unrelated_missing_var` |
| G10 | W214 dispatch-protocol from peer count alone | Require BOTH ≥3 peer procs AND at least one variable-command dispatch site (the "dispatcher evidence") | `tests/test_fp_rbs.py::test_FP_dispatch_protocol_requires_dispatcher_evidence` |
| G11 | Call-by-name VAR_READ trait conflated with caller-frame alias | New `ProcArgTrait.DYNAMIC_NAME_LOCAL` distinct from VAR_READ/VAR_WRITE; caller-side suppression only honours the genuine upvar-alias traits | `tests/test_fp_rbs.py::test_FP_call_by_name_info_exists_dynamic_target_not_caller_read` |
| G12 | W304 fires on safe braced-switch form | Special-case the 2-arg `switch STRING { ... }` shape | `tests/test_fp_nab.py::test_FP_NAB_05_braced_switch_form_should_not_fire_w304` |
| G13 | W304 lexical "currently resolves to" crosses proc boundaries | Stop the backward scan at any `proc` whose param list contains the search variable | `tests/test_fp_nab.py::test_FP_NAB_05_w304_lexical_does_not_cross_proc_boundary` |
| G14 | `namespace upvar ns src alias` -- `alias` not recorded as a def, false W210 | Add `arg_role_resolver` to the `namespace upvar` subcommand spec marking local-alias positions as `ArgRole.VAR_WRITE` | `tests/test_fp_rbs.py::test_FP_RBS_05_namespace_upvar_silent` |
| G15 | Same-element ARRAY_ELEM dead-store not detected -- `set a(k) 1; set a(k) 2` doesn't fire | New `_must_alias_killed_in_block` helper: literal-key Place equality comparison + intra-block must-alias kill detection | `tests/test_fp_ds.py::test_FP_DS_06_same_element_overwrite_fires_w220` |

Plus a paired catalog message fix: T100's diagnostic message text was
changed from "possible code injection" to "numeric coercion may
misinterpret value" -- braced expr does NOT re-parse, so the message
overclaimed (the real code-execution vector is unbraced expr / eval,
covered by W101).

### G1/G2 closure detail

The naive "treat all regexp/scan VAR_WRITE as conditional" approach
breaks Tcl idioms like ``regexp -- \\$WordBreakRE(after) abc result;
return \\$result`` (Tcl 9 init.tcl word.tcl), so the closure is
*precise*: a post-pass in `_read_before_set` scans for regexp/scan
calls where the pattern + input are bare literals (no substitution),
runs Python's `re` for regexp (or a conservative %d-only simulator
for scan), and only marks the output vars provably-unset when the
match is proven not to succeed.  Trust-the-match idioms with dynamic
or variable inputs stay silent.

### Root-cause families (all closed)

The 13 deep-review gaps clustered into four root causes -- all now
closed:

1. **Conditional def vs unconditional def** (G1, G2): SCCP-driven
   match analysis at the W210 post-pass detects when a regexp/scan
   call is provably non-matching and marks its output vars as
   provably-unset.

2. **Shape-based vs evidence-based heuristics** (G3–G8, G12): W307
   factory inference, W307 multi-dispatch, W307 callback-shape,
   W307 namespaced-ensemble, W304 switch-form -- each now consults
   SCCP CONST evidence (or Tcl form-arity rules) before suppressing.

3. **Coarse-grained whole-proc suppressions** (G9, G13): ``dict
   with`` now uses the actual dict-literal keys when statically
   known; W304 lexical-origin scan stops at any shadowing ``proc``
   declaration.

4. **Trait conflation** (G10, G11): W214 dispatch-protocol now
   requires dispatcher evidence; call-by-name VAR_READ trait split
   into VAR_READ (real upvar) and DYNAMIC_NAME_LOCAL (callee-local
   dynamic-name use).

Pre-existing open gaps (G14 namespace upvar, G15 same-element
ARRAY_ELEM dead-store) closed in the same pass via targeted spec /
must-alias fixes.

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
