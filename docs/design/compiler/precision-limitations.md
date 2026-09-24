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
Slice 8's audit of every position an existence read can reach (VT8.5)
found three positions with no synthetic statement at all, so **both** an
existence and a value read there are invisible — not a precision loss but
a miscompile, verified against `tclsh` 8.6.18 (each pair below is the
original's printed output, then the optimised program's):

- **A script body nested in a substitution** (`[catch {…}]`, `[eval {…}]`,
  `[lmap v {1} {…}]`) — records no read or write of the outer frame's
  names at all (#2231): `set x 1; puts [catch {unset x}]` loses `set x 1`
  to O109 / O126.
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
scan-order fix — the nested-substitution case is tracked as #2231 and
named for the interface contract's slice 9 (nested writes in expressions);
`uplevel 0` (#2261) and the loop header's list word (#2262) are not yet
assigned to a slice. Extend the synthetic-statement placement (or, for
`uplevel 0`, model the body as reading and writing the *current* frame
rather than a nested one) when one of these is the motivating case.

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
