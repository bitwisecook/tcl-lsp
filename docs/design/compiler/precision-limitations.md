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

## Accepted — `upvar` alias identity is name-based, not relational

Two distinct local alias names can genuinely alias the same caller variable
(`upvar 1 $x date; upvar 1 $x date2`), but only when both resolve the *same*
`$x` — a relational fact the analysis does not track. `overlap`
(`rust/tcl-compiler/src/place.rs`) therefore keeps `UpvarAlias` ↔ `UpvarAlias`
of different names overlapping: rare, and sound in the suppress-only direction.

The consequence is a false negative rather than a false positive: a genuinely
dead write through one alias is not reported when an unrelated alias in the same
frame is read.
