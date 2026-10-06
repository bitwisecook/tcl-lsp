# Known precision limitations

Places where the compiler is deliberately less precise than it could be, or
less precise than it should be. Each entry says what is imprecise, why it
matters, and where the code is.

Two kinds of entry appear here:

- **Open** — a real gap that should be closed when a motivating case or the
  enabling substrate arrives.
- **Accepted** — a limitation kept on purpose because the precise alternative
  is net-negative on real code. These are recorded so the trade-off is not
  re-litigated from scratch, and so a future change that shifts the trade-off
  knows what it is overturning.

Ground truth throughout is C tclsh 9.0.3, cross-checked against 8.x wherever the
behaviour is dialect-sensitive.

---

## Open — the interval domain covers only additive and multiplicative operators

`rust/tcl-compiler/src/intervals.rs` transfers intervals through `Add`, `Sub`,
`Mul`, and negation; comparisons yield the boolean interval. Division, modulo,
shifts, bitwise operators, and exponentiation all fall through to TOP.

That is sound — TOP is the conservative answer — and the extension is *additive*
precision with no false-positive risk, because a narrower interval can only
confirm an access in range where TOP already produced no finding. The cost is
that a genuinely out-of-range access computed through `/`, `%`, or a shift is
never proved out of range, so W230–W233 miss it.

Why it has not been done: the corpus impact is near zero. Clean libraries have
no provable divide-by-zero, and `$x % $n` narrowing to `[0, n-1]` can only
confirm what TOP already left silent. Correct interval arithmetic for five-plus
operators, including sign and zero handling, is real work for a gain nothing
currently observed motivates. Extend it when a concrete buggy-code case appears.

## Open — `Place::base()` discards dynamism

`Place::base()` (`rust/tcl-compiler/src/place.rs`) projects an `ArrayElem` or
`DictPath` place to its whole-variable identity by calling
`array_whole(name, ns, observed)`. That constructor takes only those three
fields, so the `dynamic` flag and `name_reads` on the original place are dropped.

`base()` is the join point of `overlap`, so a consumer that reasons about a base
place loses the knowledge that the name itself was computed (`set $X(k) …`). No
current consumer depends on that, which is why it has not bitten — but a future
one would silently lose the alias uncertainty rather than over-approximating,
which is the unsound direction. Carry `dynamic` and `name_reads` through `base()`
before adding a consumer that reads them.

## Accepted — dynamic array-index reads are suppress-only

`set a($i) 1` reads `$i`, and tclsh errors when `i` is unset, so in principle the
index variable belongs in the SSA use set and a never-set index should fire W210.
`rust/tcl-compiler/src/place_bridge.rs` does record the index read as a place, but
that recovery is used **only to suppress** dead-store and unused findings; it is
not promoted into read-before-set or shimmer.

Promoting it is sound in the abstract and net-negative in practice. A dynamic
array index in real code is almost always a loop or conditional variable
(`set arr($key) …`, `set DATA([list … $ip]) …`) that *is* set but not *provably*
so, so promoting the read surfaces the pre-existing loop-carried read-before-set
limitation on the index variable, and turns the index into a tracked use that the
shimmer pass then reports on. A genuinely unset index is a rare typo. The
suppress-only direction is the safe one and stays.

## Accepted — class registration is not execution-aware

`handle_oo_class_command` (`rust/tcl-compiler/src/analyser/handlers.rs`)
registers a class from the syntactic form of its creation command, with no gate
on whether that command can ever run. A class created inside `if {0} { … }` or
inside a proc that is never called does not exist at runtime — tclsh errors
`invalid command name "Foo"` on any use of it — so the over-broad registration
masks a genuine diagnostic.

The precise alternative cannot be built from static information available here.
Nothing statically distinguishes "never reachable" (`if {0}`, an unrun proc) from
the *dominant* deferred-but-does-run pattern: classes defined inside a proc that
is called, inside a `namespace eval`, or under
`if {[package vsatisfies …]} { oo::class create … }`. Real corpora are full of
the latter. Narrowing registration would drop known-class status for legitimate
objects and fire W307/W308 false positives across them, to catch the rare
never-run-class typo. A sound fix needs interprocedural call-reachability — does
this definition ever execute? — which is a much larger effort for a low payoff.

The conservative register-anything-defined behaviour is the right trade-off.

## Accepted — a regexp match past the dissector's cap declines rather than approximates

`tcl-regex`'s dissection (VT5.3) walks a repeat's iterations and a
concatenation's items in loops with a backward finish table, skips a
subtree without a capture, and closes an unbounded repeat's reach with a
worklist — but it still stops rather than approximate once a pattern's
structure exceeds its depth cap, and the engine's own fuel and depth
limits (`ExecLimits`) can also stop a search outright. Each of the three
ways the engine can finish — `Matched`, `NoMatch`, or `Stopped` — is
answered honestly: a stopped search is never folded as a no-match (an
exhausted-fuel decline answering `0` was the bug the typed three-way
result fixes), and `regexp` / `regsub` / `switch -regexp` / `lsearch
-regexp` raise the command's own real error for it
(`error while matching regular expression: …`), matching what `tclsh`
does when a pattern is too expensive to run.

Why the cap is not simply raised: a regular expression's worst-case
matching cost is exponential in the source `tclsh` links against too, so
raising the cap moves the cliff edge rather than removing it, and every
release the profile spans must agree on the answer — a capture the cap
lets through on one release and cuts off on another is a soundness bug
disguised as a precision one. The `AnalysisMatch::charge` bound keeps a
compile's cost proportional to its declared length squared, and the
pattern cache is bounded (4 MiB, coldest evicted first) rather than
grown, for the same reason: precision here trades against the budget
every other route shares, not against effort available to spend once.

## Accepted — `upvar` alias identity is name-based, not relational

Two distinct local alias names can genuinely alias the same caller variable
(`upvar 1 $x date; upvar 1 $x date2`), but only when both resolve the *same*
`$x` — a relational fact the analysis does not track. `overlap`
(`rust/tcl-compiler/src/place.rs`) therefore keeps `UpvarAlias` ↔ `UpvarAlias`
of different names overlapping: rare, and sound in the suppress-only direction.

The consequence is a false negative rather than a false positive: a genuinely
dead write through one alias is not reported when an unrelated alias in the same
frame is read.

## Open — a read inside some nested or cross-frame bodies is not recorded

`ir_helpers::variable_read_effects_from_commands` and the SSA's own use
scan see a nested command's read or write only where the lowering places a
synthetic statement for it: a condition's `<cond>`, a value word's or a
`return` word's own `<upvar-invalidate>`, and a host statement's own uses.
An audit of every position an existence read can reach found three
positions with no synthetic statement at all, so **both** an
existence and a value read there are invisible — not a precision loss but
a miscompile, verified against `tclsh` 8.6.18 (each pair below is the
original's printed output, then the optimised program's):

- **A script body nested in a substitution** (`[eval {…}]`,
  `[lmap v {1} {…}]`, an `if` arm or a loop body, `[namespace eval …]`,
  `[apply …]`) — records no read or write of the outer frame's names at
  all (#2323): `set x 1; puts [eval {info exists x}]; set x 2` loses
  `set x 1` to O109 and prints `0` where tclsh prints `1`, and `set x 1;
  puts [foreach v 1 {incr x}]; puts $x` prints `1` where tclsh prints `2`.
  The one script a substitution runs once, in this frame, whatever it
  completes with — the protected script of a `catch` and the body of a
  `try`, which the clause grammar names — is recorded where its text is
  known, brace-quoted or a quoted word that substitutes nothing
  (`[catch "incr x"]`): `set x 1; puts [catch {unset x}]` keeps `set x 1`,
  and `set x 1; set c [catch {incr x}]; if {$x == 2} …` is not decided on
  the value `x` held before the body. The script stops at its first error,
  so each place it writes may keep what it held, and the statement reads
  that as well: `set c old` stays ahead of `set r [catch {lassign {x y z} a
  b c} m]` where `b` may be an array. A script that is run-time data
  (`[catch $script]`, `[catch "incr $name"]`) may write any name, so it
  puts a barrier ahead of the statement, as the statement form does.
- **An `uplevel 0 {…}` body** (#2261) — that is the *current* frame, not a
  nested one, so its reads and writes are the caller's, but nothing records
  them:
  `proc p {} {set x 1; uplevel 0 {puts $x}; set x 2; puts $x}` prints `1`
  then `2`; with `set x 1` removed as dead (O109) the rewrite raises
  `can't read "x": no such variable`.
- **A `foreach` list word's own substitution** (#2262) — the list
  expression's side effects are real but not materialised as a use of what
  it reads,
  so a later read of a name the expression itself mutated is forwarded
  from its stale prior value instead: `proc p {} {set n 1; foreach v
  [incr n] {}; puts $n}` prints `2` (`incr n` runs once, as the list
  word); the optimised program prints `1`, O102 having forwarded `n`'s
  value from before the loop header ran.

Why it has not been done: each position needs the lowering to model a body
it does not open a synthetic statement for at all, which is more than a
scan-order fix — the bodies a substitution runs other than those of `catch`
and `try` are tracked as #2323; `uplevel 0` (#2261) and the loop header's
list word (#2262) are tracked on their own. Extend the synthetic-statement placement (or, for
`uplevel 0`, model the body as reading and writing the *current* frame
rather than a nested one) when one of these is the motivating case.

## Open — a statement's nested writes are evaluated for an assignment or an `expr` only

The solver evaluates the writes a statement's own `[…]` substitutions make —
the ordered evaluation state under `LocalWrites` — only where the statement is
an assignment of an expression, an `expr` on its own, or an assignment of one
command substitution whose command is on the expression engine's route or a
registry-owned one (`LatticeDriver::evaluate_embedded`): `set r [expr …]`,
`set a [incr n]`, `set c [catch {…} m]`. A `puts` argument, a `return`, a
branch condition and any other command's value keep the effect-free policy,
which declines a nested write that runs: `puts [expr {$x + [incr x] + $x}];
puts $x` folds nothing and forwards nothing past the statement, though it
prints `5` and `2`, `if {[catch {error boom} m]} {puts $m}` leaves `m`
unknown, and `set r [expr {$x + [incr x] + $x}]; puts $r; puts $x` folds both
reads. That is sound, and short of what the assignment form proves.

Two shapes are left undecided as well. A nested command that reads a variable
no statement records as a use (`[string length $y]`, `[incr x $y]`) leaves the
expression unevaluated: the synthetic call and the host record the reads of the
places the words write and of the expression's own variables, not of a nested
command's own words. And at a script's top level, where a `catch` stays one
call, `catch {expr {[incr x] + [error mid]}}` is not evaluated and leaves `x`
unknown; in a procedure, where the `catch` is lowered into blocks, the error
ends the evaluation with its prefix and `x` is 2.

Why it has not been done: the first shape needs the host statement to state
the reads of its own substitutions, which is more than a policy change.
Extend the host list, and the reads the call records, when one of these is
the motivating case.

## Open — a constant the solver proves at a φ is not inlined into a read

The proved-read rewrite (O100's "Inline the constant value of 'x' proved at
this read", `run_load_forwarding` in `optimiser/propagation.rs`) inlines the
value of a version a statement defines, and skips a read of a φ's version;
the name-keyed projection the other O100 forms read drops a variable whose
versions hold different constants. So `proc p {} {set x 1; catch {expr
{[incr x] + [error mid]}} msg; puts $x}` keeps `puts $x`, though `tcl
explore --show sccp` proves `x#3 = const(2)` there, the φ where the `catch`
ends; and `proc p {c} {set x 1; if {$c} {set x 2} else {set x 2}; puts $x}`
keeps it too. The branch folds read the lattice, so `if {$x == 2}` in the
read's place is decided (I230, O101). That is sound — the read stays — and
short of what the lattice proves.

Why it has not been done: the def-use consumer knows the φ's version, but
the rewrite asks for a defining statement whose span it rewrites, which a φ
does not have. Extend `run_load_forwarding` to a φ's version when a program
motivates it.

## Open — an element of a place a store preserved holds no value

`scan` goes on past a store it cannot make and preserves the place that store
failed on. The solver gives the definitions of that place's elements — the
fan of a whole-variable write — no value, since a definition takes only the
stores to its own place (`defs_from_placed` in `value_transfer.rs`). So after
`array set a {k keep}; set b old; catch {scan {1 2} {%d %d} a b}`, `$a(k)` is
unknown and `if {$a(k) eq "keep"} …` is not decided, where after `catch
{lassign {x y} b a}`, which stops at `a` and never reaches it, I230 reports the
condition always true and O101 folds it. Sound, and short of what tclsh
leaves.

Why it has not been done: an element's definition would take the stores to
its base where every one of them is a `Preserve`, a rule the driver does not
state yet for any command. Extend `defs_from_placed` when a program motivates
it.

## Accepted — a nested unbind's kill is not a definition

A nested `[unset x]` (D167) is recorded as reading the version of `x` it
observes — the fix every other existence-read position (a condition, a
value word, a `return` word) takes — but never as *killing* it. A killing
definition would have to sit on the synthetic statement the lowering
places **before** its host statement, so the host word's own reads would
see the killed version too: `set y $x[unset x]` would draw a spurious
W210 on `$x` and read no value, where `tclsh` 8.4.20 to 9.1b0 read `$x`
before the unset runs and then remove it, in source order within the one
word. `proc p {} {set x 1; puts [unset x]; puts $x}` therefore still
rewrites `puts $x` to `puts 1` (O102), where every release raises `can't
read "x"` on the second `puts`.

The suppress-only direction — an existence read keeps the store live,
never the reverse — means the unmodelled kill can only under-report a
read-before-set past a nested unbind, never delete a store a real read
still needs. Modelling the kill precisely needs a second synthetic
statement per nested unbind (one for the read it makes, ordered before its
host word; one for the kill, ordered after it), which no other existence
read needs and which the placement machinery does not have a slot for
today. Tracked as #2263.

## Open — a write through `::name` does not alias the top-level `name`

In top-level code a plain name is the global of that name, and the solver
gives it the footing it gives a procedure's local: the constant it
propagates, and the narrowing a test proves in the arm the test guards
(`refinable_values` in `sccp.rs`), hold until a call to a command the file
does not define gives the name a fresh version, and a name one of the file's
procedures declares `global` is neither propagated nor narrowed. A write
through the qualified spelling in the same code is not read as a write to the
plain name:

```tcl
set z a
set ::z c
set w $z
puts $w
```

prints `c` under tclsh 8.4 to 9.1, and `tcl opt` forwards `z`'s literal
(O102), removes `set w` (O109) and inlines `w` (O100), so the rewritten
program prints `a`. With `set z [gets stdin]` and the three statements inside
`if {$z eq "a"}`, the arm's narrowing gives `w` the same `a`, which O100
inlines, and a test `if {$z eq "c"}` after `set ::z c` there is reported
always false (I230) and folded, where tclsh takes it.

Why it has not been done: the escaping set (`escaping_names` in `sccp.rs`,
over `var_observability::analyse_var_observability`) has no entry for a name
the same body writes through its qualified spelling, and the narrowing reads
the same set, so neither is less sound than the other; the fix is that entry,
for both at once. Tracked as #2370.

## Open — a procedure's implicit return value is not a recorded use

Every Tcl command returns a value, and the last one a procedure body runs
supplies the call's own result when nothing calls `return` explicitly.
Nothing in the SSA records that implicit read: `proc p {} {set y 5; set y}`
prints `5` under every release (`set y`, the bare form, reads `y`), but O126
sees only that `y`'s one definition has no recorded use and removes
`set y 5` (VT8.5's find), leaving `set y` to read an undefined `y` — the
rewritten procedure raises `can't read "y"` where the original returns `5`.

Why it has not been done: the CFG's terminator for a body with no explicit
`return` does not carry an operand the way `Terminator::Return { value }`
does for an explicit one, so there is no use site to attach; giving the
implicit return path a value operand is a small CFG change with a
correctness payoff (every procedure without a trailing `return`, which
idiomatic Tcl leans on heavily) disproportionate to how the case was
found — auditing existence-read positions for slice 8. Tracked as #2264;
not assigned to a slice.

## Open — a procedure that calls one defined after it has no transfer summary

A procedure's transfer summary (`interprocedural/transfer.rs`, slice 13)
says what a call to it does to its caller's places; a caller's lattice
applies it, and without one the call widens every place it may write. A
procedure has none when it reaches code the module cannot see, and the flow
graph marks a call to a procedure the file defines *later* as exactly that:
the source-order timeline gives the call the unseen-call marker
(`SyntheticMarker::UnseenCall`), since at the definition the callee is not
yet bound. So in

```tcl
proc twice {name} {upvar 1 $name w; bump w; bump w}
proc bump {name} {upvar 1 $name v; incr v}
```

`twice` has no summary, where with `bump` defined first it has one, its
place bound afterwards; mutual recursion therefore never summarises, while
a procedure that calls itself does. The answer is sound — the call keeps
the widening every call to unseen code has — and only precision is lost.

Why it has not been done: the marker is the flow graph's source-order rule
for every caller, not the summary's, and a procedure body runs after the
whole file in the ordinary case; reading a forward call as a call to the
procedure the file defines is a change to that rule, with its own witnesses.
Not assigned to a slice.

## Open — a module that rebinds any builtin summarises no procedure

A module that rebinds a builtin — a `proc` named like one, a `rename` or an
alias onto one, or a rebinding whose subject the scan cannot name — has no
transfer summaries at all (`ModuleCommandMutations::rebinds_builtins`). The
flow graph lowers a builtin to a typed statement (`set`, `incr`, `expr`) with
no call in it, so a call to the module's replacement there is invisible to
the summary's call walk, which could then miss a frame the replacement
reaches. Every call to a procedure of such a module keeps the widening.

Why it has not been done: telling which typed statements a rebinding moves
is the lowering's question, answered per command; the conservative rule
holds until a rebinding module motivates the precise one. Not assigned to a
slice.
