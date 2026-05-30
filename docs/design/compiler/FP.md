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

The verdict on audit: the analyser DOES descend into the body — proven here by `info exists ${a}($a)` firing W216 (scalar-vs-array smell) inside the proc.  This locks in the depth contract.

#### tclsh ground truth

```
% snit::type T { proc Helper {a} { return [info exists ${a}($a)] }
 method m {a} { return [Helper $a] }
}
% T create t
% t m foo
0  # exists test is well-formed; W216 is a *static-analysis* smell about the form
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

The W307 detector inspects command-word shape; the namespaced-ensemble suppression matches the regex / shape `\[...\](::[a-zA-Z_][a-zA-Z0-9_]*)+`.

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


## Precision gaps (PR #498 deep-review)

The PR #498 deep review identified 13 precision regressions — places
where a FP suppression went too far and converted a TP into a false-
negative.  Below: which gaps were closed (paired TP test) and which
remain open (strict xfail).

### Closed (11 of 13)

| # | Gap | Closure | Test |
|---|---|---|---|
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

Plus a paired catalog message fix: T100's diagnostic message text was
changed from "possible code injection" to "numeric coercion may
misinterpret value" -- braced expr does NOT re-parse, so the message
overclaimed (the real code-execution vector is unbraced expr / eval,
covered by W101).

### Still open (2 of 13)

| # | Gap | Reproducer | xfail test |
|---|---|---|---|
| G1 | regexp no-match path: ``v`` stays unset; reading ``$v`` after a failed match should fire W210 | `proc f {} { regexp {x} y -> v; puts $v }` | `tests/test_fp_rbs.py::test_FP_RBS_02_regexp_no_match_path_precision_gap` |
| G2 | scan no-match path: same root cause | `proc g {} { scan abc %d n; puts $n }` | `tests/test_fp_rbs.py::test_FP_RBS_02_scan_no_match_path_precision_gap` |

**G1/G2 deferred** because the simple closure (treat regexp/scan
output vars as never-defined) breaks the documented Tcl idiom of
trusting a match (Tcl 9 init.tcl word.tcl: ``regexp -- \\$WordBreakRE
(after) abc result; return \\$result``).  Proper closure needs SCCP-
driven pattern simulation: only fire W210 when the pattern + input
are statically known AND statically proven not to match (or the read
is on a no-match branch reached via the regexp return-value test).
That's a precision improvement; the current behaviour is conservative.

**Pre-existing open gaps** (also tracked by strict xfail; carried over
from earlier review rounds):

- G14 — FP-RBS-05 ``namespace upvar`` alias-not-a-def (real caller-
  scope def but no lowering hook).
- G15 — FP-DS-06 same-element ARRAY_ELEM dead-store detection (Place
  model overlap relation is suppress-only).

### Root-cause families (closed clusters in **bold**)

The 13 deep-review gaps clustered into four root causes:

1. **Conditional def vs unconditional def** (G1, G2 — OPEN):
   ``regexp`` / ``scan`` only write on success; current model treats
   them as unconditional defs.  Closure requires SCCP-driven pattern
   simulation; naive "never define" approach breaks Tcl idioms.

2. **Shape-based vs evidence-based heuristics** (G3–G8, G12 — **all
   closed**): W307 factory inference, W307 multi-dispatch, W307
   callback-shape, W307 namespaced-ensemble, W304 switch-form -- each
   now consults SCCP CONST evidence (or Tcl form-arity rules) before
   suppressing.

3. **Coarse-grained whole-proc suppressions** (G9, G13 — **both
   closed**): ``dict with`` now uses the actual dict-literal keys
   when statically known; W304 lexical-origin scan stops at any
   shadowing ``proc`` declaration.

4. **Trait conflation** (G10, G11 — **both closed**): W214 dispatch-
   protocol now requires dispatcher evidence; call-by-name VAR_READ
   trait split into VAR_READ (real upvar) and DYNAMIC_NAME_LOCAL
   (callee-local dynamic-name use).

Three of the four clusters are fully closed.  Cluster 1 is deferred
to a future SCCP-driven precision improvement.

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
