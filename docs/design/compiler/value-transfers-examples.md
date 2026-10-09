# Value transfers — worked examples

One program per optimisation and diagnostic code the value axis touches,
and per rung, each with what the tool reports on it and the rule behind the
report; the open defects the examples witness; and the declarations behind
the examples as they are written in the Rust command registry and in
`.tclspec` packs. The four programs at the top of the interface contract in
[value-transfers.md](value-transfers.md) are the shortest of these;
[value-evaluation.md](value-evaluation.md) states the routes the
declarations name, and
[value-transfers-migration.md](value-transfers-migration.md) indexes the
codes the programs exercise.

> **How the observations were made.** Every comment on a program line is an
> observation, not an assertion: the program was run through
> `tcl diag FILE --json` and `tcl opt FILE --profile full` (a single
> optimiser pass) built from this tree, and through tclsh 8.4.20, 8.5.19,
> 8.6.18, 9.0.4, and 9.1.0 for ground truth. A program with no dialect note
> runs under `--dialect tcl8.6`, the CLI's fallback, and one written with
> `when` under `--dialect f5-irules`; a dialect in parentheses at the start
> of a comment is the `--dialect` that line was run under on its own. A
> finding no comment names is incidental to the example: an unused
> variable in a fragment, or a read of a variable the fragment never sets.

## How to read an example

Each code has up to three programs:

- **the baseline** — the shape the code is defined on, and what the tool
  reports on it;
- **a computed constant** — the same shape with a value the shared lattice
  proves by computing it (an `append`-built string, a `lappend`-built
  list, a `string range` result, a variable holding an option name), and
  what the tool makes of it;
- **a rewrite that must stay sound** — a rewrite or finding a naive
  transfer would get wrong, and the rule that prevents it.

An entry whose title ends in *rung* is one of the designed rungs rather
than a single code — its programs are the ones its consumers share, in the
same three roles — and it sits in the section where its consumers' codes
live. Where the optimiser and a diagnostic answer differently on the same
program, the example says so.

## Open defects the examples witness

| Issue | Program | The rule it breaks |
|---|---|---|
| [#2143](https://github.com/bitwisecook/tcl-lsp/issues/2143) | § W102: `Configure port 8080 …` then `puts [port ignored]` | a procedure a factory materialises is a command the analyser knows: its W214 is anchored at the factory call, as it should be, but the call to it is W123 |
| [#2439](https://github.com/bitwisecook/tcl-lsp/issues/2439) | § O110, O111, O116, the proc-level summaries, *Completion paths* (under `--dialect tcl8.4`), W201, and the literal-only option checks | `tcl opt`'s summary lists the rewrites it applied; it lists O100 and O102 forwards the text does not carry |
| [#2440](https://github.com/bitwisecook/tcl-lsp/issues/2440) | § W121: `set m 255.0.255.0` inside `when CLIENT_ACCEPTED { … }` | a finding is anchored once, at a span that holds what it reports; W121 reports the literal mask again at the enclosing body's `{` |
| [#2441](https://github.com/bitwisecook/tcl-lsp/issues/2441) | § *Bounded-loop enumeration*: `foreach x {} {}` then `puts $x` | a read of a place the existence rung holds unbound is a read before set: I230 decides `[info exists x]` there from the fact, and W210 does not report the read |

## Optimisations

### O100 · propagate constant variables

```tcl
set retries 1
incr retries
puts "$retries"            ;# O100 inlines 2
```

```tcl
set acc ""
append acc foo
append acc bar
puts $acc                  ;# O104 folds the chain to `set acc foobar`, and O100 inlines `foobar`
```

The cell update makes `acc` `Const("foobar")` at the read, so the chain
folds (O104) and the read is forwarded in the same pass; the code is O100
rather than O102 because the defining statement is a computed write.

```tcl
set n 1
set result [incr n]        ;# `set n 1` stays; O100 inlines 2 into both reads below
puts $result
puts $n
```

The nested `incr` reads `n` as an SSA use, so the store stays; the result
`2` is forwarded into `puts $result` while the `incr` itself is preserved
(permission 2 of the three permissions). Every release prints 2 and 2,
optimised or not.

### O101 · fold constant integer expressions

```tcl
set x [expr {2 + 3}]       ;# O101 rewrites to `set x 5`
```

```tcl
set s foo
append s bar
set n [expr {[string length $s] * 2}]   ;# not folded: the nested `[string length $s]` declines `not-exact`
set t [expr {"x"}]                      ;# O101 rewrites to `set t x`
set z [expr {0 && [error never]}]       ;# `z` is proven 0, the evaluator short-circuiting; the assignment is kept as written
```

The evaluator stops at `0 &&`, so the substitution on the right is never
reached, and `"x"` folds because the boundary carries the engine's full
value. The nested `[string length $s]` is resolved through the registry
when it is reached, but the fused assignment lowers without the read of
`s` inside it (an `if` condition over the same expression records it, and
I230 decides that condition), so `s` has no value there and `n` stays
unproven. Every release from 8.4 to 9.1 computes `12`, `x`, and `0` for
`n`, `t`, and `z`.

### O102 · forward a single reaching literal load

```tcl
set n 7
puts $n                    ;# O102 forwards 7 and O109 removes the store
```

A literal load is a literal load: a computed write forwards as O100, never
as O102.

### O103 · fold static procedure calls

```tcl
proc double {n} { expr {$n * 2} }
set x [double 21]          ;# O103 folds the call to 42
```

```tcl
proc label {} { set s abc; append s def; return $s }
set x [label]              ;# not folded; O104 folds the body's chain to `set s abcdef`
```

Both of O103's paths — the argument-sensitive re-run, and the summary path
over `label`'s seedless lattice, which holds `s` at `abcdef` where it
returns — fold only a pure callee, and the purity scan takes `append`'s
write for an effect whatever variable it names, so `[label]` is not
folded.

### O104 · fold string build chains

```tcl
set s "hello"
append s " world"          ;# O104 folds to `set s {hello world}`, and W104 flags the space in the value
```

```tcl
set s hello
set p again
append s $p                ;# O104 folds the chain to `set s helloagain`, and O100 inlines it into the `puts`
puts $s
```

The classifier dispatches on the resolved cell update instead of three
command names, and the chain folds through the lattice value at the last
write, `helloagain`. With `set p { again}` the exact-value ingress keeps
the leading space, and the chain folds to `set s {hello again}`.

### O105 · redundant computation (GVN/CSE)

```tcl
llength $items
llength $items             ;# the shape the pass documents: top level, a CSE_CANDIDATE command, plain-word arguments
```

For this program, for a literal list, and for a list from `$argv` or
`[gets stdin]`, under `full` and under `aggressive`, the CLI reports no
O105. The same value is not the same observable computation.

### O106 · hoist loop-invariant computations

```tcl
foreach i $list {
    set n [llength $list]  ;# the shape the pass documents: a top-level loop, an invariant pure command
    puts "$i $n"
}
```

As for O105, the CLI reports nothing for this shape. Hoisting must not
introduce execution on the zero-trip path or move an error.

### O107 · eliminate unreachable code

```tcl
proc p {x} {
    return $x
    puts "never reached"   ;# O107 removes it
}
```

```tcl
set x b
switch -- $x {
    a { puts A }
    b { puts B }
    default { puts D }
}                          ;# I231 on arm `a`, never selected, and on arm `b`, after which no arm is reached; O112 folds the switch to `puts B`
```

The CFG subject of a whole-variable `switch` is a `Raw` operand that the
lattice reads as the one variable it names, so the dispatch chain decides
per arm and the dead arm bodies leave `executable_blocks`: I231 reports
them, and O112, reading the same decided branches — for an opaque form,
the same selection record — and keeping no `Env` of its own, folds the
whole statement, which leaves O107 no arm to remove. Where the option scan
may read the subject — a release that may be before 8.5, or pattern and
body words on any release — a variable subject with no `--` before it
stays one statement and its selection record decides it: I231 names the
arms never selected and O112 folds the statement, but no arm has a block
for O107 to remove. The observation follows the closing brace: inside a
braced `switch` body, `;#` begins a pattern word, not a comment.

### O108 · eliminate transitively dead code

```tcl
proc report {x} {
    set a 1                ;# O108 deletes it once `b` goes
    set b [expr {$a + 1}]  ;# O126 deletes it, and W211 reports it
    return $x
}
```

```tcl
proc p {} {
    set unused "\{"
    lappend unused value   ;# kept: `lappend` is never deleted, and a call of `p` raises "unmatched open brace in list"
    return 0
}
```

The second program stays as it is: a dead target with pure argument words
does not make the command removable. Removal needs the totality proof of
permission 3: old value well-formed for the operation, place proven bound,
no trace.

### O109 · eliminate dead stores

```tcl
set limit 1                ;# O109 deletes it, and W220 reports it
set limit 2
puts $limit
```

The storage contract's witnesses keep their stores: the store a nested
`[incr n]` reads (§ O100), and the stores ahead of a `regexp` that does
not match (§ W210), through `catch` as well (§ *Completion paths*).
Deleting a quoted store removes its quotes with it: `set p "again"`
leaves no `"` behind.

### O110 · canonicalise expressions

```tcl
proc p {x} {
    set y [expr {$x + 1 + 2}]   ;# nothing: O110 leaves the sum as written
    return $y
}
```

```tcl
set x 10000000000000000.0
expr {$x + 1 + 2}          ;# 10000000000000002.0 from 8.5, 1e+16 on 8.4; the text keeps `$x` though the summary lists an O102 forward (#2439)
expr {$x + 3}              ;# 10000000000000004.0 from 8.5, 1e+16 on 8.4; the text keeps `$x` though the summary lists an O102 forward (#2439)
```

With `x` unknown, regrouping `$x + 1 + 2` as `$x + 3` changes the result
for a double, as the second program shows, so O110 leaves the sum as
written: reassociation consumes the type and target proofs, and the
floating-point pair is pinned by a regression test
(`reassociation_refuses_an_unproven_float_term`). With `x` known, `tcl opt`
leaves both expressions of the second program as written.

### O111 · brace-expression hints

```tcl
set x 1
set y [expr $x + 1]        ;# W100, and O111 at the same span; the text keeps `$x` though the summary lists an O102 forward (#2439)
```

O111 is a producer over the unbraced-expression fact:
`brace_expr_hints` (`rust/tcl-lsp-core/src/diagnostic_report.rs`) emits one
at the span of every W100 the analyser emitted and reads no policy, so it
does not depend on W100 surviving presentation, and policy decides the two
codes independently: disabling W100 does not silence O111 — the rule
[diagnostic-policy.md](diagnostic-policy.md) § *Producers that change*
states. `tcl diag` runs the producer too and, keeping the optimiser off,
holds its O111 as an `optimiser-off` suppression, which
`--show-suppressed` lists.

### O112 · eliminate constant-condition compounds

```tcl
if {1} { puts yes } else { puts no }   ;# I230, and O112 rewrites to `puts yes`
```

```tcl
set acc ""
append acc foo
if {$acc eq "foo"} { puts yes } else { puts no }   ;# I230; O104 folds the chain, O101 folds the condition and O107 empties the else
```

The cell update decides the condition: I230 reports it, and `tcl opt`
folds it through O101 and O107 rather than O112, leaving
`if {1} { puts yes } else {  }`.

### O113 · strength reduction

```tcl
proc p {key} {
    if {$key % 8} { return spill }   ;# O113 rewrites to `$key & 7`
    return [expr {$key ** 2}]   ;# O110 rewrites to `$key * $key`
}
```

Integer versus float and the target's arithmetic rules stay explicit, and
Tcl identities are never transferred to BPF-Tcl.

### O114 · the `incr` idiom

```tcl
set count 0
set count [expr {$count + 1}]   ;# O108 and O109 delete both assignments: `count` is never read, and W220 reports this one
```

`count` is never read here, so the dead-code passes remove both
assignments and leave O114 nothing to rewrite; the loop under
*Bounded-loop enumeration* below shows O114 rewriting
`set i [expr {$i + 1}]` to `incr i`. The rewrite's error,
initial-existence, and trace behaviour must agree with `incr`'s.

### O115 · redundant nested `expr`

```tcl
proc p {x} {
    if {[expr {$x > 0}]} { return pos }   ;# O115 removes the nested `expr`, and W114 reports it
    return neg
}
```

### O116 · fold a constant `list`

```tcl
puts [list a b c]          ;# O116 folds to `{a b c}`
```

```tcl
set a x
append a y
puts [list $a b]           ;# not folded: O104 folds the chain to `set a xy`, and the text keeps `$a` here though the summary lists an O100 inline (#2439)
```

The cell update gives `a` its value at the read, but the `list` call over
it is not folded; every release prints `xy b`.

### O117 · `[string length $s] == 0`

```tcl
proc p {s} {
    if {[string length $s] == 0} { return empty }   ;# O117 rewrites to `$s eq ""`
    return full
}
```

### O118 · fold a constant `lindex`

```tcl
set val [lindex {red green blue} 1]   ;# O118 folds to `green`
```

```tcl
set l {}
lappend l red
lappend l green
set val [lindex $l 1]      ;# not folded; O130 folds the chain to `set l {red green}`
```

The chain folds, and the `lindex` over the folded list is not folded in
the same pass.

### O119 · pack consecutive `set` literals

```tcl
proc p {} {
    set a 1
    set b 2
    set c 3                ;# O119 packs the three into `lassign {1 2 3} a b c`
    return [list $a $b $c]
}
```

The packed form must agree on results, traces, partial failure, and
scope, not only on final values.

### O120 · `eq`/`ne` for string comparison

```tcl
proc p {name} {
    if {$name == "admin"} { return 1 }   ;# O120 rewrites to `eq`, and W110 reports the `==`
    return 0
}
```

W110 is not an edit certificate.

### O121, O122, O123 · recursion

```tcl
proc walk {node acc} {
  if {$node eq ""} { return $acc }
  set acc [combine $acc [walk [left $node] {}]]   ;# the non-tail self-call holds O122 off
  return [walk [right $node] $acc]                ;# O121 rewrites to `tailcall`
}
proc fact {n acc} {
  if {$n <= 1} { return $acc }
  return [fact [expr {$n-1}] [expr {$n*$acc}]]   ;# O122 converts `fact` to a `while` loop; a bracketed argument is one argument
}
proc countdown {n} {
    if {$n <= 1} { return 1 } else { countdown [expr {$n - 1}] }   ;# O122 converts `countdown` to a `while` loop
}
proc fact2 {n} {
    if {$n <= 1} { return 1 } else { return [expr {$n * [fact2 [expr {$n - 1}]]}] }   ;# O123, a hint with no rewrite
}
```

```tcl
proc walk {node acc} {
  if {$node eq ""} { return $acc }
  set acc [expr {$acc + [walk [left $node] 0]}]   ;# the self-call inside the braced `expr` holds O122 off, as the plain spelling's does
  return [walk [right $node] $acc]   ;# O121 rewrites to `tailcall`
}
```

The tail-call passes read no values. The gate that chooses between O121
and O122 reads the self-call through the expression owner's bridge, so a
self-call inside a braced `expr` holds O122 off as the plain spelling does.

### O124 · unused iRule procs

```tcl
proc legacy {} { return 1 }   ;# O124 comments the procedure out
when HTTP_REQUEST { pool main }
```

Whether a call is reachable is `ProcSummary::calls`' question on the
call-graph axis, and no value fact answers it.

### O125 · sink assignments into a decision block

```tcl
proc p {ok} {
    set msg "error"        ;# O125 sinks `msg` into the else branch
    if {$ok} { return } else { puts $msg }
}
```

### O126 · unused assignments

```tcl
proc handle {} {
    set unused 42          ;# O126 deletes it, and W211 reports it
    puts done
}
```

The unused fact is separate from the producer's effects.

### O127 · inline a single-use assignment

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]    ;# O127 inlines it into `log local0. [set uri [HTTP::uri]]`
    log local0. $uri
}
```

A constant use takes O100 instead.

### O128 · end-offset indices

```tcl
proc p {L s} {
    set last [lindex $L [expr {[llength $L] - 1}]]              ;# O128 rewrites the index to `end`
    set last_char [string index $s [expr {[string length $s] - 1}]]   ;# O128 rewrites the index to `end`
    return "$last $last_char"
}
```

Its length-position table is debt on the `arg_roles` axis
([value-transfers-migration.md](value-transfers-migration.md) § *The
ratchet over unreviewed files*).

### O129 · fold a pure builtin substitution

```tcl
set s [string range foobarbaz 3 6]   ;# O129 folds to `barb`
```

```tcl
set h [binary format H* 00ff]        ;# nothing: neither `$h` nor the `string length` below is folded
puts [string length $h]
```

`binary format` has a direct route (`BinaryFormatSemantics`, over
`tcl_cmd_core::binary::format`) whose value carries byte-array
representation evidence, and a non-printable value is never spliced into
source, so `$h` stays; the `string length` over it is not folded either,
though every release prints 2. The rule that a known result never
replaces a writing producer keeps `[incr n]` out of O129.

### O130 · fold `lappend` chains

```tcl
set l {}
lappend l a b              ;# O130 folds to `set l {a b}`
```

```tcl
lappend l a                ;# O130 folds the chain to `set l {a b}` from the absent cell
lappend l b
puts $l
```

The initial cell's existence is a fact — absent, so `lappend` creates it —
and the chain folds from `lappend l a`; every release prints `a b`.

### The ordered evaluation state · rung

```tcl
set x 1
puts [expr {0 && [incr x]}]   ;# O101 folds the expression to 0
puts $x                       ;# not forwarded, and every release prints 1
```

The program prints `0` and `1` in every release, optimised or not: the
right operand of `0 &&` is never reached, so the increment never runs, and
O101 folds the expression. The read after it is not forwarded: a `puts`
argument keeps the effect-free policy (below), so the call that carries
`[incr x]` gives `x` a definition the lattice holds unknown.

```tcl
set x 1
set r [expr {$x + [incr x] + $x}]   ;# r is 5 and x is 2 for every consumer
puts $r                             ;# O100 inlines 5
puts $x                             ;# O100 inlines 2, the nested increment's store
```

```tcl
set x 1
set r [expr {$x + [set x 10] + $x}] ;# r is 21 and x is 10
puts $r                             ;# O100 inlines 21
puts $x                             ;# O100 inlines 10, and `set x 1` stays
```

The first prints `5` then `2` and the second `21` then `10` in every
release, optimised or not. A nested `[incr x]` or `[set x 10]` is a write of
the frame, made in the expression's order: a read through the `variable`
service consults the state's `writes` first, so `$x` after `[incr x]` is `2`,
the expression is `5`, and the store the later `puts` reads is the nested
one's. The SSA gives a statement one version of a name, so the statement
whose words read `x` before, between and after the nested `[set x 10]`
reads the version before it wherever a word reads the place, and no word
of the statement is an operand to forward: `set x 1` stays.

```tcl
set x 1
puts [expr {$x + [incr x] + $x}]   ;# not folded: the host is a `puts`, which keeps the effect-free policy
puts $x                            ;# not forwarded; the program prints 5 and 2, optimised or not
```

A `puts` argument, a `return`, a condition and a command's value other than
an `expr` keep the effect-free policy (`NestedPolicy::EffectFreeOnly`),
which declines a nested write that runs: the statement is not folded, and
the definition its word effects give the place keeps a later read off the
earlier version.

```tcl
proc p {} {
    set n 1                ;# kept: the nested increment's read is a use; O100 folds the `return` to 3
    set r [expr {$n + [incr n]}]
    return $r
}
proc q {} {
    set n 1                ;# kept, with no W211; O100 folds the `return` to 5
    set r [expr {[incr n] + [incr n]}]
    return $r
}
```

`p` is `3` and `q` is `5` in every release, optimised or not. The read is
inside a braced `expr`, so neither the dead-store guard nor the
unused-variable check would see it if it were not an SSA use of the version
it reads. `NestedPolicy::LocalWrites` is the policy that admits the nested
increments, and an error inside the expression ends the evaluation with the
writes so far: in a procedure, where the `catch` is lowered into blocks,
`set x 1; catch {expr {[incr x] + [error mid]}} msg` leaves `x` 2 — a
following `if {$x == 2}` is I230 — and at a script's top level, where the
`catch` stays one call, `x` is unknown and the same test is not decided.

### Bounded-loop enumeration · rung

```tcl
for {set i 0} {$i < 5} {incr i} {}
if {$i == 5} { puts five } else { puts other }   ;# I230; O101 folds the condition and O107 removes the else
```

```tcl
set t 0
for {set i 0} {$i < 4} {incr i} { incr t $i }
if {$t == 6} { puts six } else { puts other }    ;# I230; O101 and O107 — the accumulator survives the run
```

Both decide through the loop's enumeration, which states `i` 5 and `t` 6 on
the loop's exit edges: `tcl diag` reports I230 and `tcl opt` folds the
branch from the one fact.

```tcl
for {set i 0} {$i < 10} {incr i} { if {$i == 3} break }
if {$i == 3} { puts three } else { puts other }   ;# I230; O101 and O107
```

```tcl
foreach x {} {}
puts $x                    ;# nothing: no W210, though every release raises `can't read "x"`
```

`i` is `3` and `puts $x` raises `can't read "x": no such variable` in
every release. `break` is a completion code the iteration plan absorbs and
the exit state is published on the exit edges, so the branch decides. After
the zero-iteration `foreach` the existence rung holds `x` unbound — a
condition `[info exists x]` there is I230, always false, at the top level
and in a procedure alike — but W210 does not report the read
([#2441](https://github.com/bitwisecook/tcl-lsp/issues/2441)).

```tcl
set n 0
for {set i 0} {$i < 3} {incr i} { set i [expr {$i + 1}] ; incr n }
if {$i == 4} { puts four } else { puts other }   ;# I230; O114, then O101 and O107 decide it true
```

`i` is `4` and `n` is `2` in every release, which is what the tool answers:
a body that writes the loop variable is enumerated, not modelled, and every
statement of the body applies its own registry-owned evaluation over the
enumeration's state.

### The correlated finite-set limit · rung

```tcl
set r 0
foreach a {1 2} { set r [expr {$a * $a}] }
if {$r == 4} { puts four } else { puts other }   ;# I230, O101 and O107, from the loop's exit state
```

`r` is `4` in every release. `a` is one distinct SSA value with the finite
set `{1, 2}`, so per-member evaluation answers `{1, 4}` — never `{1, 2, 4}`
— inside the loop, and the loop's exit state decides the branch.

```tcl
set x 0
foreach {a b} {1 10 2 20} { incr x [expr {$b / $a}] }
if {$x == 20} { puts twenty } else { puts other }   ;# I230; O101 and O107 decide it from the loop's exit state
set y 0
foreach {a b} {1 20 2 10} { incr y [expr {$b / $a}] }
if {$y == 25} { puts twentyfive } else { puts other }   ;# I230; O101 and O107 decide it from the loop's exit state
```

`x` is `20` and `y` is `25` in every release. The two loops give `a` the
set `{1, 2}` and `b` the set `{10, 20}` either way, so no reading of the
sets alone can separate them: pairing members by position answers
`{10, 10}` for both, and the cartesian product `{5, 10, 20}` is sound for
both and exact for neither. Two distinct finite inputs decline with
`DeclineReason::CorrelatedSets`, and the definitions widen; it is the
loop's enumeration, running each iteration in order, that answers `20` and
`25`, not the finite-set lift.

### Proc-level transfer summaries · rung

```tcl
proc bump {name} {
    upvar 1 $name v        ;# O100 rewrites this to `upvar 1 n v`: the one call site is the whole call set
    incr v
}
set n 1
bump n
if {$n == 2} { puts two } else { puts other }   ;# I230; O101 folds the test and O107 empties the else arm
```

`n` is `2` in every release. The rewrite specialises the callee to its call
site, and under whole-module trust the module's call sites are the
procedure's whole call set
([interprocedural-call-site-seeding.md](interprocedural-call-site-seeding.md)).
A second caller that passes another name suppresses it: `bump n; bump other`
leaves the body alone, and so does `set cmd bump; $cmd m`, a dispatch the
scan resolves by value to a call `bump m`
([§ *How an indirection is resolved*](interprocedural-call-site-seeding.md#how-an-indirection-is-resolved)).
A `package provide` in the file, or a `rename` or `interp alias` that
touches `bump`, withdraws the seed whole-module; a frame-shifting `uplevel`
leaves every value a local holds unknown to the scan, so that a dispatch
word such as `$cmd` then withdraws it too. A caller in another file that
`source`s this one as a plain file is the residual gap
([§ *Known residual gaps*](interprocedural-call-site-seeding.md#known-residual-gaps)).

```tcl
proc reset {name} { upvar 1 $name v ; unset v }   ;# O100 rewrites this body too, to `upvar 1 m v`
set m 1
reset m
puts [info exists m]       ;# the call leaves `m` unbound; no rewrite folds an `info exists` result
proc g {} { set ::counter 5 }
g
puts $::counter            ;# not forwarded: the call binds `::counter` and states no value
proc rec {n} {
    if {$n <= 0} { return 0 }
    return [expr {$n + [rec [expr {$n - 1}]]}]
}
puts [rec 4]               ;# O103 folds it to `puts 10`
```

`info exists m` is `0`, `::counter` is `5`, and `rec 4` is `10` in every
release. `reset`'s body is specialised to its one call site as `bump`'s is.
The `Name` parameter carries the callee's outcomes to the caller's place,
so `reset m` unbinds `m`; the summary's `globals` carry `::counter`'s
binding, whose value it does not state; and the argument-sensitive O103
path re-runs the callee under the call's seeds, and its exit reading
re-runs the call a return's expression makes, which is what folds
`[rec 4]`.

```tcl
proc bump {name} { upvar 1 $name v ; incr v }
set n 1
bump n
set other 10
bump other
puts "$n $other"           ;# the body is left alone; the text keeps both reads though the summary lists two O100 inlines (#2439)
```

`2 11` in every release. One context-insensitive summary per procedure is
what both call sites apply, each with a re-run under its own place's
value, so the caller's places are updated without specialising the callee
to either of them, and `bump absent` keeps its release split — an error
under 8.4, `1` with `absent` bound from 8.5.

## Diagnostics

### I230 · constant branch condition

```tcl
set x 1
if {$x == 1} { puts one } else { puts other }   ;# I230, and O112 rewrites to `puts one`
```

```tcl
set acc ""
append acc foo
if {$acc eq "foo"} { puts yes } else { puts no }            ;# I230; O104 folds the chain, and O101 and O107 decide the branch
set s abcdef
if {[string length $s] == 6} { puts six } else { puts other } ;# I230; O101 and O107
```

Both conditions of the second program decide: the first through the cell
update, the second through the command substitution in the condition,
which the expression route resolves through the registry when the
evaluation reaches it.

### I231 · constant switch arm

```tcl
switch -- 1 {
    1       { puts "one" }
    2       { puts "two" }
    default { puts "other" }
}                          ;# I231 on arm `1`, after which no arm is reached; O112 folds the switch to `puts "one"`
```

```tcl
set x b
switch $x { a { puts A } b { puts B } default { puts D } }   ;# I231: arm `a` never matches and the arms after `b` are unreachable; O112 folds to `puts B`
switch -glob -- $x { a* { puts A } b* { puts B } }           ;# I231: arm `a*` is never selected; O112 folds to `puts B`
```

The whole-variable `Raw` case decides in the CFG, and the opaque `-glob`
form gets a selection fact from `tcl_cmd_core::switch` that I231, O112, and
the analyser consume together. The observation on the first program follows
the closing brace, as under § O107.

### Predicate refinement · rung

```tcl
proc p {} {
    if {![info exists x]} { set x 0 }
    puts $x                ;# no W210 — the existence rung holds x bound on both paths into the read
}
```

The existence guard is the precedent: the existence rung refines a guard's
edges in its own domain, and a W210 read asks it last.

```tcl
proc p {x} {
    if {$x eq "a"} {
        if {$x eq "b"} { puts both }   ;# I230 "always false"; O101 folds it and O107 drops the arm
    }
}
proc q {x} {
    switch -- $x {
        a { return [string length $x] }
        default { return 0 }
    }
}                          ;# the arm holds x as a; nothing is reported or rewritten
proc r {x} {
    if {[string is integer $x]} { return [expr {$x + 1}] }   ;# nothing — without -strict the test proves no type
    return 0
}
proc s {x} {
    if {$x in {a b c}} {
        if {$x eq "d"} { puts no }     ;# I230 "always false"; O101 and O107
    }
}
```

Each test publishes an `EdgeRefinement` on the edge it decides, and the
solver narrows `x` by it in every block each path into which crosses that
edge — the table in [value-transfers.md](value-transfers.md)
§ *Predicate refinement* gives each shape's row. `p`'s inner condition
decides false, so I230 and O101 fire on it and O107 drops the arm; `q`'s
arm entry holds `x` as `a`, though `[string length $x]` there is not
folded; `r`'s test proves nothing, since without `-strict` the empty string
passes it and stays a string — `[string is integer -strict $x]` carries an
integer type, never a value; `s`'s true edge carries the finite set
`{a b c}`, so the inner test decides false. `s` is 8.5 onwards, where the
`in` operator exists: `$x in {a b c}` is a syntax error under 8.4.

```tcl
proc p {x} {
    if {$x == 1} { return [string length $x] }   ;# nothing — x may be 1.0, whose length is 3
    return no
}
proc q {x} {
    if {$x == 8} { return $x }
    return no
}
```

`p 1.0` is `3` in every release and `q 08` is `no` under 8.4, 8.5, and 8.6
and `08` under 9.0 and 9.1. A numeric `==` refines the `Range` domain to a
point and the `Type` domain to numeric and never the exact value — that is
why `==` is not `eq` — the leading-zero release split is a decision about
which edge is taken and not about the string, and an externally mutable
place (`is_externally_mutable`) is never refined at all.

### W124 · invalid IP literal

```tcl
set addr "10.0.0.256"      ;# W124; O102 forwards it and O109 removes the store
puts $addr
```

```tcl
set addr 10.0.0
append addr .256           ;# W124 at the computed string; O104 folds the chain and O100 inlines `10.0.0.256`
puts $addr
```

Every `Const(String)` in the lattice is checked, so the computed address
is reported as the literal one is. The finding is anchored at the
offending literal's own bytes inside the defining statement when the source
spells it out, and otherwise at the whole statement: no word of the second
program spells `10.0.0.256`, so its W124 is at the `append`.

### W121 · non-contiguous subnet mask

```tcl
when CLIENT_ACCEPTED {
  set m 255.0.255.0        ;# W121 here, at the read of `$m`, and again at the body's `{` (#2440)
  if {[IP::addr [IP::client_addr] mask $m] eq "10.0.0.0"} { reject }
}
```

```tcl
when CLIENT_ACCEPTED {
  set m 255.0
  append m .255.0          ;# W121 at the read of `$m`, from the computed mask; O104 folds the chain to `set m 255.0.255.0`
  if {[IP::addr [IP::client_addr] mask $m] eq "10.0.0.0"} { reject }
}
```

W121 is one of the literal-only checks that run again over a word the
lattice proves (§ *W127, W137, …* below): the `$m` in the `IP::addr` call
the condition makes is the mask the cell update proves, so the computed
mask is reported at its use as the literal one is. The literal is reported
at the `set` too, by the check over the word itself; no word of the second
program spells the computed mask. The finding at the `when` body's `{` in
the first program repeats the literal's
([#2440](https://github.com/bitwisecook/tcl-lsp/issues/2440)).

### W233 · division by a provably zero divisor

```tcl
proc p {x} { return [expr {$x / 0}] }   ;# W233
```

```tcl
proc p {x} {
    set z [string range 1000 3 3]
    return [expr {$x / $z}]    ;# W233: the divisor is proven 0, and O129 folds `z` to 0
}
```

`z` is the exact value the direct route proves for `string range`
(`StringRangeSemantics`), the interval domain seeds the point `[0, 0]` from
the lattice, and W233 reports the division from the same fact the
optimiser folds.

### W230, W231, W232 · constant index out of range

```tcl
set tail [lindex {a b c} end-5]    ;# W230; O118 folds it to `{}`
set xs {a b c}
lset xs 5 X                        ;# W231; every release raises here
set last [string index "abc" end-5] ;# W232; O129 folds it to `{}`
```

```tcl
set l {a b c}
set tail [lindex $l 9]     ;# W230; O118 folds it to `{}`
set s abc
set last [string index $s 9]   ;# W232; O129 folds it to `{}`
```

```tcl
set xs {}
lappend xs a b c
lset xs 2 X                ;# no W231; O130 folds the chain to `set xs {a b c}`
```

The literal-only index checks run again over a container the lattice
proves — `$l` and `$s` here — and report at the literal index it makes
checkable, inside a command substitution as on a line of its own; an index
whose `$var` value the interval checks bound is theirs, so a site is
reported once. The container length after a cell update is the value the
update leaves, so `lset xs 2 X` on the three-element list is in range.

### W210 · read before set

```tcl
proc p {} {
    regexp xy zz m         ;# no match leaves `m` unset, and W210 reports the read below
    puts $m
}
```

```tcl
proc p {} {
    regexp {(x)(y)} zz a b ;# no match leaves both unset, and W210 reports both reads below
    puts "$a $b"
}
```

```tcl
proc p {} {
    set a before
    set b before
    regexp {(x)(y)} zz a b ;# no W220; both stores stay, and O100 inlines `before before` into the `puts`
    puts "$a $b"
}
```

`regexp` evaluates the match through the engine (`RegexpSemantics`), and a
no-match outcome *preserves* each target: in the first two programs nothing
bound the targets before the call, so the reads after it are W210, and the
third keeps its stores, whose values reach the `puts` — `p` prints
`before before` in every release.

### W211, W213, W214, W220, H300 · variable lifecycle

```tcl
proc p {} {
    set x 1                ;# no W211 — the read in the dead arm counts as a use
    if {0} { puts $x }     ;# I230, and O112 removes the `if`
}
proc q {} { unset maybe_defined }         ;# W213
proc greet {name greeting} { puts "Hello, $name" }   ;# W214 on `greeting`
set y 1                    ;# W220, and O109 deletes it
set y 2
puts $y
set z 1                    ;# W211; H300 and W220 report the repeat below
set z 1
```

W211 counts the read in the dead arm as a use. W213 reports an `unset` of a
name the procedure never binds, W214 a parameter the body never reads, W220
a store a later store overwrites unread, and H300 an assignment that
repeats the static value the one before it wrote.

### Existence · rung

```tcl
proc p {} {
    if {[info exists x]} { puts $x }   ;# I230 "always false"; O101 folds the condition to 0 and O107 empties the arm
}
proc q {} {
    if {[info exists x]} { puts ok }
    puts $x                ;# W210 — a read outside the guard still reports
}
proc r {a} {
    if {[info exists a]} { puts yes } else { puts no }   ;# I230 "always true"; O101 and O107
}
```

The whole-body scan decides a name no statement assigns and a parameter
every call binds: `r 1` is `yes` in every release, and the existence read
is a use of `a`, so W214 does not report it. The guarded read in `p` draws
no W210: it is in the guard's region (`SccpResult::guarded`), the block its
edge enters and every block that one dominates.

```tcl
proc p {} {
    set x 1
    unset x
    if {[info exists x]} { puts yes } else { puts no }   ;# I230 "always false"; O101 and O107 decide it
}
```

`p` is `no` in every release. The unbind is a storage outcome, so the fact
at the condition is `Existence::Unbound`, `[info exists x]` answers `0`
through the expression route's `nested` service, and the condition decides
inside the fixed point, so the dead arm leaves `executable_blocks`.

```tcl
proc p {} {
    set x 1                ;# kept, with no W211
    if {[info exists x]} { puts yes }
}
proc q {} {
    incr n                 ;# kept
    if {[info exists n]} { puts yes }
}
proc r {} {
    set a 1                ;# O109 deletes it once O100 folds the next store to `set b 2`
    set b [expr {$a + 1}]  ;# kept as `set b 2`, with no W211
    if {[info exists b]} { puts yes }
}
proc s {c} {
    set x 1
    if {$c} { unset x }
    puts $x                ;# W210, and no S100
}
```

`p` and `r` print `yes` in every release, optimised or not; `q` is the
absent-cell split — `can't read "n": no such variable` under 8.4, `yes`
from 8.5. An existence read is a use of the binding, so none of those
stores is removable while one remains, and an unbind statement is never
removed at all; W213 reads the same fact, so an `unset` of an `Unbound`
place is definite and of a `MayBound` place is "may not exist"; and an
unbound version is not a typed value, so `s`'s phi is not a representation
merge and S100 is silent on it.

### Completion paths · rung

```tcl
proc p {} {
    set a before           ;# kept, with no W220
    if {[catch {regexp {(x)(y)} zz a b} m]} { puts $m }
    puts $a
}
```

`p` is `before` in every release, optimised or not. The no-match outcome
preserves both targets on the normal path, so the store is read and stays.

```tcl
set a old
array set b {k keep}
catch {lassign {new second} a b} m   ;# `set a old` stays in every release, with no W220
puts "$a $m"
```

The `catch` is `1` from 8.5 with `a` equal to `new` and the message
`can't set "b": variable is array`, because `lassign` wrote `a` before it
failed on `b`; under 8.4 it is `1` with `a` still `old` and the message
`invalid command name "lassign"`. The outcome is `Error { written: 1, … }`
from 8.5, and the prefix rule is what proves the store dead — `written` is
the proof. Under `--dialect tcl8.4` a write by a command the profile lacks
is no write (`command_is_unavailable_here`): W002 and W123 report
`lassign`, the store stays, and the summary `tcl opt` prints lists an O102
forward of `a` the text does not carry
([#2439](https://github.com/bitwisecook/tcl-lsp/issues/2439)).

The store stays in this program, dead though it is from 8.5, because the
last command of a `catch` body supplies the value the `catch` stores, whose
names the dead-store passes leave alone, and at a script's top level the
`catch` stays one call, whose word effects read every version the body may
overwrite. Where the body is lowered into blocks and `lassign` is not the
last command it runs — in a `try`, or in a procedure's
`catch {lassign {new second} a b; set z 1} m` — W211 reports `set a old`
and O126 deletes it.

```tcl
proc p {} {
    set f 0                ;# W220, and O109 deletes it: nothing reads it
    try {
        error boom
    } finally {
        set f 1            ;# W220, and O109 deletes it: nothing reads it
    }
    return $f              ;# never runs: the error resumes after `finally`
}
```

`p` raises `boom` from 8.6 and `invalid command name "try"` under 8.4 and
8.5. `finally` runs on every path, so its body is reachable and no read of
`f` is of an unset `f`; the error resumes after the clause, so `return $f`
never runs, and both stores are dead because nothing reads them. The
`Handlers` protocol gives `finally` its edge from every completion path.

### W126 · non-channel value in channel position

```tcl
puts "output.txt" "hello"             ;# W126
set ch [string cat out put.txt]
puts $ch "hello"                      ;# W126 — the type lattice knows `string cat` returns a string
```

The semantic type travels with the value: `string cat` returns a string,
which names no channel, and W126 reports it as it reports the literal.

### W123, W307, W308 · dispatch heads

```tcl
unknownCmd 1               ;# W123
set cmd unknown
append cmd Cmd
$cmd 1                     ;# W307; O104 and O100 rewrite the call to `unknownCmd 1`
```

```tcl
set cmd [gets stdin]
$cmd hello                 ;# W307 and T100
set cmd puts
$cmd hello                 ;# no W307 — the proven literal suppresses it; O102 forwards `puts`
set cmd pu
append cmd ts
$cmd hello                 ;# no W307: O104 folds the head to `puts`, and O100 inlines it
```

```tcl
oo::class create Point { method x {} { return 0 } }
set p [Point new]
$p distance                ;# W308
set m dist
append m ance
$p $m                      ;# nothing: W308 does not read the computed `distance`
```

A head the lattice proves to name a command the analyser knows
suppresses W307, whether the literal was assigned or computed, and the
optimiser rewrites the call to name the command. A proven head that names
no command the analyser knows stays W307 rather than W123: `unknownCmd`
is reported as a head it cannot analyse, though the optimiser rewrites
the call to `unknownCmd 1`. W308 reads the method word as written, so the
computed `distance` is not reported.

### W102 · `subst` on a computed operand

```tcl
subst $x                             ;# W102 — any [cmd] and $var in the string will be evaluated; add -nocommands -novariables
subst -nocommands $x                 ;# W102 — any $var; add -novariables
subst -nocommands -novariables $x    ;# nothing — only backslashes run
set opt -novariables
subst $opt {hello $name}             ;# nothing — the operand is the braced literal; the computed word is a switch
subst -backslashes $tmpl             ;# (tcl9.1) nothing — the positive family names the only kind that runs
subst -commands $x                   ;# (tcl9.1) W102 — any [cmd]; no switch advice, since the families cannot be combined
```

```tcl
set opt -novariables
subst $opt $x                        ;# W102 — any [cmd]; add -nocommands: the proven `-novariables` is read
```

The check asks `CommandRegistry::substitutions_performed` for the kinds a
call runs, and finds its advice by asking what each declared option would
do to the call. Over the lattice, each call the unit holds a template-word
plan for (`SccpResult::template_plans`) is read again with the switch
values the lattice proves, and its finding replaces the walk's at the
template word: the second program warns of `[cmd]` alone and advises
`-nocommands`, as `subst -novariables $x` does. A switch word the lattice
does not prove leaves the call unreadable, so the answer is every kind and
the advice nothing.

```tcl
set name world
set greeting [subst -nocommands {hello $name}]   ;# nothing — the fold reaches lowering's const map
puts $greeting
proc Configure {name default description} {
    proc $name {x} [subst -nocommands {return $default}]
}
Configure port 8080 {the port}       ;# W214 on `x` of proc `::port`, anchored at this call
puts [port ignored]                  ;# W123: the analyser does not know the procedure the factory makes (#2143)
```

`subst -nocommands {hello $name}` is `hello world` and `port ignored` is
`8080` in every release. The fold of the first reads the kinds the registry
answers for the call: exactly `SUBST_NOCOMMANDS_KINDS` (`[cmd]` off, `$var`
and backslashes on), which a computed switch word never answers, since an
unreadable call answers every kind. The factory specialisation reads the
same answer from the template-word plan (`literal_template_plan`): a
braced `-nocommands` template materialises the child `::port`, whose W214
is anchored at the `Configure` call, since the child has no span of its
own. The analyser does not know the command the factory materialises, so
the call `port ignored` is W123
([#2143](https://github.com/bitwisecook/tcl-lsp/issues/2143)).

### S100, S101, S102, S103, S110 · representation

```tcl
set v 5
expr {$v + 1}
lindex $v 0                ;# S100 — numeric intrep read as a list
set d [dict create a 1]
puts [lindex $d 0]         ;# S100 — dict intrep read as a list
set items [list 1 2 3]
foreach item $items { puts [expr {$item + 0}]; puts [lindex $item 0] }   ;# S101
set total 0
foreach item {1 2 3} { set total [expr {$total + 1}]; set total [string range $total 0 end] }   ;# S101 and S102
set a [lrepeat 1000 x]
set b $a
lappend b y                ;# nothing: no S103
set bin [binary format H* ff00]
set u [string toupper $bin] ;# S110 — the registry's return type marks the byte array
```

The exact value, the semantic type, and the representation evidence are
three facts. S103 reports a mutation that copies a value another holder
still reads, so it needs both holders live: `a` is never read after
`lappend b y`, and the copy is not reported. S110 fires from the byte-array
type the registry gives `binary format`'s result, not from a lattice
string.

### T100–T106, IRULE3001–3004, W313 · taint

```tcl
set cmd [gets stdin]
eval $cmd                  ;# T100
set name [gets stdin]
puts $name                 ;# T101
set pattern [gets stdin]
set matches [glob $pattern]   ;# T102 (and W304)
set pat [gets stdin]
regexp $pat abc            ;# T103 (and T102, W304)
set host [gets stdin]
socket $host 80            ;# T104
set child [interp create -safe]
interp eval $child $cmd    ;# T105 (and W312)
proc p {userPath} { file delete $userPath }   ;# W313 (and W304)
```

```tcl
set cmd [gets stdin]
if {0} { eval $cmd }       ;# I230, no T100 — taint reads applied reachability
```

```tcl
when HTTP_REQUEST {
  set host [HTTP::host]
  HTTP::respond 200 content "<h1>$host</h1>"   ;# IRULE3001
  log local0. "Host: $host"                    ;# IRULE3003
}
when HTTP_RESPONSE {
  set val [HTTP::header value X-Custom]
  HTTP::header replace X-Reply $val            ;# IRULE3002
}
when HTTP_REQUEST {
  set target [HTTP::header value Location]
  HTTP::redirect $target                       ;# IRULE3004
}
```

```tcl
when HTTP_REQUEST {
  set u [URI::encode [HTTP::uri]]
  set v [URI::encode $u]     ;# T106 — $u already carries the encoder's colour
  HTTP::redirect "https://example.com/?next=$v"
}
```

Taint never reads values; colour flows through write outcomes, and only
applied reachability drops a flow, so the `eval` in the dead arm is not
T100.

### IRULE3101, IRULE3102, IRULE3103 · URI setters and getters

```tcl
when HTTP_REQUEST { HTTP::uri "newpath" }    ;# IRULE3101
when HTTP_REQUEST {
  set p /a
  HTTP::path $p            ;# no IRULE3101 — the proven `/a` starts with `/`; O102 forwards it
}
when HTTP_REQUEST {
  if {[string match "*admin*" [HTTP::path]]} { reject }   ;# IRULE3102
}
when HTTP_REQUEST {
  set uri [HTTP::uri]
  set parts [split $uri "?"]   ;# IRULE3103
  if {[lindex $parts 0] eq "/admin"} { reject }
}
```

The setter check reads the lattice constant, so a subject proven to start
with `/` is clean.

### IRULE1005–1008, 1201, 1202, 4002, 4004, 5002, 5004 · protocol and events

```tcl
when HTTP_REQUEST_DATA { log local0. [HTTP::payload] }   ;# IRULE1005
when HTTP_REQUEST { set p [HTTP::payload] }              ;# IRULE1006
when CLIENT_ACCEPTED { TCP::collect 1024 }               ;# no IRULE1007: the `TCP::release` below matches it
when CLIENT_ACCEPTED { TCP::release }                    ;# no IRULE1008: the `TCP::collect` above matches it
when HTTP_REQUEST { HTTP::respond 200; HTTP::header insert X-Custom val }   ;# IRULE1201
when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/blocked"} { HTTP::respond 403 }
  HTTP::redirect "https://example.com/"                  ;# IRULE1202
}
when RULE_INIT { set static::debug 0 }                   ;# IRULE4002
when HTTP_REQUEST { set pool_name "main_pool" }          ;# IRULE4004
when HTTP_REQUEST { drop; log local0. "after drop" }     ;# IRULE5002
when DNS_REQUEST { DNS::return "1.2.3.4"; log local0. "still runs" }   ;# IRULE5004
```

On its own line each of the two `CLIENT_ACCEPTED` events is IRULE1007 or
IRULE1008; in one rule the `TCP::collect` and the `TCP::release` pair up,
and neither is reported.

```tcl
when HTTP_REQUEST {
  if {0} { HTTP::respond 200 }
  HTTP::header insert X-Custom val   ;# no IRULE1201 — the dead arm commits nothing
}
when HTTP_REQUEST {
  set pool_name [string range main_pool_v2 0 8]   ;# IRULE4004 — the computed value is hoistable
}
```

The response-commit walk consumes applied reachability, so the respond in
the dead arm commits nothing, and IRULE4004 sees a computed constant as
hoistable.

### W201 · manual path concatenation

```tcl
set dir /tmp
set filename report.txt
set path "$dir/$filename"  ;# W201; the text keeps both reads though the summary lists two O102 forwards (#2439)
```

W201 reads the rendered value's properties, so a computed path is checked
as a literal one is.

### W303 · ReDoS pattern

```tcl
proc p {input} { regexp {(a+)+$} $input }   ;# W303
```

```tcl
proc p {input} {
    set re {(a+)+$}
    regexp $re $input      ;# W303 on the proven pattern; O100 propagates it into the call
}
```

The pattern-role check runs again over a pattern the lattice proves, so
the pattern held in `re` is reported at `$re`.

### W240, W241, W242 · loop conditions

```tcl
while {0} { puts "never runs" }   ;# W240 and I230
while {1} { puts "forever" }      ;# W241
proc p {} {
    set i 0
    while {$i < 10} { puts $i }   ;# W241: nothing in the loop changes `i`
}
```

```tcl
set n 0
while {$n} { puts "never runs" }  ;# I230 and W240; O112 removes the loop
set go 1
while {$go} { puts "forever" }    ;# W241, and O101 folds the condition to 1
```

W240 and W241 read the branch fact I230 uses, so a header the solver
decides — a literal, a proven variable, or `$i < 10` with `i` proven `0`
and never changed in the loop — is W240 or W241, and W242 is left for a
header nothing decides. The top-level `while {1}` never ends, so the
`proc p` definition after it never runs, and O107 removes it.

### W127, W137, W138, W141, W145, W146, W147, W152, W200, W202 · literal-only option and value checks

```tcl
when HTTP_RESPONSE priority 5 { HTTP::version "2.0" }   ;# W127
if {[string is dict $config]} { puts dict }              ;# (tcl8.5) W137
puts [format "flags: %b" 5]                              ;# (tcl8.5) W138
proc r {path} { return -code error -errorstack {CALL load extra} "cannot read $path" }   ;# W141
puts [string l $s]                                       ;# W145
trace add variable ::config(port) {read rename write} logChange   ;# W146
set l [lsort -increasing -decreasing {b a}]              ;# nothing: `lsort` takes the last of the two
::bibtex::parse -command handle -recordcommand rec       ;# W152 and W147
set data [binary format iu 42]                           ;# (tcl8.4) W200
set data [binary format n 42]                            ;# (tcl8.4) W202
```

```tcl
when HTTP_RESPONSE priority 5 { set v 2.0; HTTP::version $v }   ;# W127 on the proven `2.0`; O102 forwards it
set cls dict; if {[string is $cls $config]} { puts dict }       ;# (tcl8.5) W137 on the proven `dict`
set f "flags: %b"; puts [format $f 5]                           ;# (tcl8.5) W138 on the proven format
proc r {path} { set es {CALL load extra}; return -code error -errorstack $es "cannot read $path" }   ;# W141 on the proven list
set sub l; puts [string $sub $s]                                ;# nothing
set ops {read rename write}; trace add variable ::config(port) $ops logChange   ;# nothing: the unknown `logChange` widens `ops` at the call
set o -decreasing; set l [lsort -increasing $o {b a}]           ;# nothing, as for the literal spelling; the text keeps `$o` though the summary lists an O102 forward (#2439)
set fmt iu; set data [binary format $fmt 42]                    ;# (tcl8.4) W200 on the proven `iu`; the text keeps `$fmt` though the summary lists an O102 forward (#2439)
set fmt n; set data [binary format $fmt 42]                     ;# (tcl8.4) W202 on the proven `n`; the text keeps `$fmt` though the summary lists an O102 forward (#2439)
```

A check the walk abstains from because a word it reads is not literal runs
again over the value the lattice proves for that word at its statement,
and reports at the word the user wrote, once, with no fix: the word is a
substitution, which a fix would replace with a constant
(`rust/tcl-compiler/src/analyser/diagnostics/proven.rs`). The words it
reads are a call statement's, those of the command substitutions a
statement or a terminator performs, and a `return`'s own, read where the
procedure leaves; a `return` that leaves the procedure writes nothing, so
the value `es` holds there is the one `set` gave it. W127, W137, W138,
W141, W200, and W202 report over the proven value here — tclsh raises
`forbidden odd-sized list for -errorstack` for `r` from 8.6, as for the
literal spelling. The other lines report nothing:

- **`string $sub $s`.** A `string` whose subcommand is computed is a
  barrier that may write a variable the source does not name — the CFG's
  reason is `string writes a source-opaque variable name` — and the
  lattice holds `sub` unproven in the frame, so W145 has no proven word to
  read.
- **`trace add variable … $ops logChange`.** The unknown `logChange`
  widens `ops` at the call; with `proc logChange {args} {}` defined, W146
  reports the proven list.
- **`lsort -increasing $o {b a}`.** The literal spelling draws no W147
  either: `lsort` takes the last of the two, and every release prints
  `b a`.

## The declarations behind the examples

Each excerpt below is a declaration as it is in the tree: a command spec in
the Rust command registry (`rust/tcl-registry/src/commands/`), or a command
in a `.tclspec` pack that `tcl spec export` loads with no notice. A spec's
`semantics` field names the value-transfer specialisation
(`rust/tcl-registry/src/value_transfer/`): `SemanticsDeclaration::Declared`
a registry-owned one, `Declined` none, and `Inherited` — the default — the
enclosing scope's, and at the outermost scope one derived from a
descriptor that states the same operation (`declaration.rs`, `derive`), or
none.

### `incr`, `append`, `lappend`: a cell update

`rust/tcl-registry/src/commands/tcl/incr_.rs`, with the fields this page
does not read left out:

```rust,ignore
CommandSpec {
    name: "incr",
    traits: Traits::FRAMELESS_RUNTIME
        | Traits::BYTE_COMPILED
        | Traits::READS_BEFORE_WRITE
        | Traits::FIRST_ARG_VARNAME,
    arity: Arity::new(1, 2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    assigns_variable_at: Some(0),
    safe_on_uninit: Some(SpecSurface::TCL85_PLUS),
    return_type: Some(TclType::Int),
    lowering_hook: Some(LoweringHookId::Incr),
    native_lowering: Some(NativeLowering::CellReadModifyWrite(CellUpdate::Increment)),
    inline_codegen_hook: Some(InlineCodegenHookId::Incr),
    ..CommandSpec::DEFAULT
}
```

The spec declares no `semantics`: the operation is already stated, and
the specialisation derives from it
(`rust/tcl-registry/src/value_transfer/declaration.rs`):

```rust,ignore
fn derive(spec: &CommandSpec, sub: Option<&SubCommand>) -> ResolvedSemantics {
    if sub.is_none()
        && let Some(NativeLowering::CellReadModifyWrite(update)) = spec.native_lowering
    {
        return ResolvedSemantics::Derived(DerivedSemantics::CellUpdate(CellUpdateSemantics {
            update,
            creates_absent: spec.safe_on_uninit,
        }));
    }
    let traits = spec.traits | sub.map_or_else(Traits::empty, |sub| sub.traits);
    if traits.contains(Traits::DESTROYS_VARIABLE) {
        return ResolvedSemantics::Derived(DerivedSemantics::Unbind(UnbindSemantics));
    }
    ResolvedSemantics::None
}
```

`append_.rs` and `lappend_.rs` declare `CellUpdate::Append` and
`CellUpdate::ListAppend` the same way. `CellUpdateSemantics`
(`value_transfer/cell_update.rs`) has one target, the first operand, read
then written; its result and target type is `Int`, `String`, or `List`;
the release axes it reads are the numeral grammar and the integer tower
for `incr`, none for `append`, and the list rendering for `lappend`; and
its catalogued evaluator is `NativeEvalId::CellIncrement`, `CellAppend`,
or `CellListAppend`, over the shared cores. `creates_absent` is the spec's
`safe_on_uninit`, which is why `bump absent` under § *Proc-level transfer
summaries* raises under 8.4 and binds `absent` from 8.5.

### `string range`, `binary format`: the direct route

The `range` subcommand in `string_.rs`:

```rust,ignore
SubCommand {
    name: "range",
    semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringRange)),
    byte_array_effect: ByteArrayEffect::Transparent,
    arity: Arity::exact(3),
    pure: true,
    return_type: Some(TclType::String),
    semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STRING_RANGE),
    const_fold: Some(fold_range_unanimous),
    const_fold_versioned: Some(fold_range),
    ..SubCommand::DEFAULT
}
```

`STRING_RANGE` is `StringRangeSemantics` (`value_transfer/builtins.rs`),
whose evaluator admits the index grammar, the character indexing, and the
source encoding (`Needs::INDEX_GRAMMAR`, `CHAR_INDEXING`,
`SOURCE_ENCODING`), resolves the index numerals under the admitted
grammar, and runs `tcl_cmd_core::string::range` over `ConstOps`.
`fold_range` is the same route run over literal words
(`value_transfer::evaluate_literal`), under the analysed release's grammar,
and `fold_range_unanimous` runs it under the answer every release gives,
so the registry folds and the lattice answer alike. A pack writes the same
subcommand's folder as a `const_fold` body that calls the real command in
the sandbox (`docs/design/spec-dsl-examples/string.tclspec`):

```tcl
const_fold {words ctx} {
    if {[llength $words] != 3} return
    lassign $words s first last
    if {![string is ascii $s]} return
    # A malformed index raises here, and an error in a hook body is an
    # abstention — the same answer the shipped folder's None gives.
    fold [string range $s $first $last]
}
```

`binary format` in `binary_.rs` declares `pure: true`,
`return_type: Some(TclType::ByteArray)`, and
`semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::BINARY_FORMAT)`.
`BinaryFormatSemantics` admits the binary field set, the crossing of
characters to bytes, and the source encoding, charges
`tcl_cmd_core::binary::format_size_bound` to the budget before it runs
`tcl_cmd_core::binary::format`, and declines a field, or a value one
takes, that the target's releases do not all pack alike. Its value
carries byte-array representation evidence, § O129 never splices a
non-printable value into source, and S110 reads the byte-array type the
registry gives the result, not the string.

### `expr`: the expression route

`expr_.rs` declares `arg_roles: &[(0, ArgRole::Expr)]`,
`return_type: Some(TclType::Numeric)`, and
`semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::EXPR)`,
where

```rust,ignore
pub static EXPR: ExpressionRoute = ExpressionRoute {
    language: LanguageProfileId::TclExpr,
};
```

The route assembles the arguments as the command specifies
(`ExpressionRoute::assemble`): one braced word is the expression text,
whose `$name` reads the engine performs through the `variable` service;
any other word reaches the engine already substituted; several words join
with one space. The driver's engine adapter evaluates it with the
analysis services — `variable`, and `nested` under the host's
`NestedPolicy` (§ *The ordered evaluation state*) — and the boundary
carries the engine's full value. A pack command names the same route with
`evaluate -expression tcl.expr`.

### `regexp`, `scan`, `lassign`: several targets, write or preserve

`regexp_.rs`, `scan_.rs`, and `lassign.rs` resolve their `VarWrite`
targets through an `arg_role_resolver` and declare
`semantics: SemanticsDeclaration::Declared(&crate::value_transfer::regex::REGEXP)`,
`…::destructure::SCAN`, and `…::destructure::LASSIGN`. Each answers one
outcome per target, in call order: `regexp`'s no-match outcome
*preserves* every target (§ W210, § *Completion paths*), `scan` writes the
targets its conversions fill and preserves the rest, and `lassign` writes
the elements in order, the empty string past the list's end; an error
outcome records how many targets were written before it — the
`Error { written: 1, … }` of § *Completion paths*.

A pack command with an authored evaluator says *preserve* itself:

```tcl
command kv::split3 {
    arity 4
    arg_role_resolver {words ctx} { role 1 VarWrite; role 2 VarWrite; role 3 VarWrite }
    arg_role_resolver_roles {VarWrite}
    return_type Int
    semantics {
        stores -targets {1 2 3} -outcome write_or_preserve
        result -semantic int
    }
    evaluate -implementation kv.split3.v1 -host bounded_tcl {
        inputs {arg 0 exact}
        depends {tcl_profile implementation_identity}
        budget {-commands 2000 -wall-clock 20 -value-bytes 65536}
        body {s} {
            set parts [split $s :]
            if {[llength $parts] != 3} {
                preserve 1; preserve 2; preserve 3   ;# silence on a declared target is a decline
                fold 0
                return
            }
            lassign $parts a b c
            write 1 $a; write 2 $b; write 3 $c
            fold 3
        }
    }
}
```

The body's three verbs are the whole protocol: `fold` states the result,
`write` and `preserve` state one outcome each, in call order, and a body
that says nothing about a declared target declines the whole answer, so
the no-match path spells all three `preserve`s out. In statement position
`kv::split3 host:port:path a b c` writes `port` into `b`, and after
`set x old; kv::split3 nocolons x y z` the lattice holds `x` at `old`. A
target declared `write_or_preserve` is a conditional write, the class
`regexp`'s targets have: the call records a use of the target's prior
value, so the `set x old` ahead of it draws no W220 and no O109 deletes
it, and the pack spells no trait for it. In value position,
`set n [kv::split3 …]`, a call whose outcome writes storage is not
substituted: the substitution has no definition for its stores.

### `switch`: selection semantics

`switch_.rs` declares `case_list: Some(&CaseListSpec::SWITCH)`,
`lowering_hook: Some(LoweringHookId::Switch)`, and
`semantics: SemanticsDeclaration::Declared(&SEMANTICS)`, where

```rust,ignore
static SEMANTICS: crate::value_transfer::selection::SwitchSemantics =
    crate::value_transfer::selection::SwitchSemantics {
        case_list: CaseListSpec::SWITCH,
        options: OPTIONS,
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
    };
```

The case-list descriptor gives the arms, the bodies, the fall-through and
the default; the selection fact runs the shared switch core
(`tcl_cmd_core::switch::{parse_options, select_analysis}`) for each member
of a proven subject over the arms' exact patterns, ordered first match, and
a pattern that cannot be evaluated declines the whole fact. I231, O112, and
the analyser read it together (§ I231, § O107).

### `unset`, `dict with`: unbind, and a structural plan

`unset_.rs` resolves its targets through `unset_arg_roles`, carries
`Traits::DESTROYS_VARIABLE`, and declares no `semantics`: the trait states
the operation, and `derive` above makes it `UnbindSemantics`, whose
outcome is an unbind of each target: § *Existence* reads it as
`Existence::Unbound`, and the `reset m` of § *Proc-level transfer
summaries* carries it to the caller's `m`. The `with`
subcommand in `dict.rs` resolves `VarWrite`, `VarRead`, and `Body` roles
and declares `Declared(&crate::value_transfer::body::DICT_WITH)`:
`DictWithSemantics` is a structural plan — one binder per key of the
dictionary the variable holds, the body run in the enclosing frame
(`FrameLevel::Relative(0)`), and the keys written back when it completes
(`Reconcile::WriteBackKeys`) — that declines when the dictionary is not
known exactly, leaving the generic transfer; it has no evaluator
(`NoRouteReason::Unauthored`).

### `subst`: which substitutions a call runs

`subst_.rs` declares `Traits::PERFORMS_SUBSTITUTION` and
`semantics: SemanticsDeclaration::Declared(&TEMPLATE)`, where

```rust,ignore
static TEMPLATE: crate::value_transfer::template::TemplateSemantics =
    crate::value_transfer::template::TemplateSemantics::new(
        "template:subst",
        OPTIONS,
        FAMILIES,
        RESERVED_TRAILING_WORDS,
    );
```

The template-word plan reads the switch words' values and answers the
kinds the call runs; W102 reads it over the lattice (§ W102), and the
factory specialisation over literal words. The options are the
option-effect descriptor of
[registry-consumer-contracts.md](registry-consumer-contracts.md)
§ *Options with semantic effects*, which a pack spells as
`docs/design/spec-dsl-examples/subst.tclspec` does:

```tcl
command subst {
    runtime_backing shipped-builtin subst
    arity 1..
    reserved_trailing_words 1

    option_effect_family negated { base all-on  combine accumulate }
    option_effect_family positive { base all-off combine accumulate \
                                    -introduced 9.1 }

    option -nobackslashes -effect {disables substitution backslashes} \
                          -family negated
    option -nocommands    -effect {disables substitution commands} \
                          -family negated
    option -novariables   -effect {disables substitution variables} \
                          -family negated
    option -backslashes   -effect {selects substitution backslashes} \
                          -family positive -introduced 9.1
    option -commands      -effect {selects substitution commands} \
                          -family positive -introduced 9.1
    option -variables     -effect {selects substitution variables} \
                          -family positive -introduced 9.1

    option_forbids -nobackslashes {-backslashes -commands -variables}
    option_forbids -nocommands {-backslashes -commands -variables}
    option_forbids -novariables {-backslashes -commands -variables}
}
```

`-introduced` is the row's own release gate: the plan reads each
combination of switch spellings at every release the profile names, a
combination that raises runs nothing, and one the releases read
differently declines `ReleaseAmbiguous`. The three `option_forbids` rows
forbid mixing the families.

### A vendor loop and a vendor collection in a shipped pack

`specs/sdc_base.tclspec` declares `foreach_in_collection` as below, the
`form` and `hover` rows left out. `analyser_hook -native Foreach` routes
the call through the analyser's `foreach` handler, and the `iterate` block
states the loop to the value axis: the binder, an iterable that is a
vendor collection handle rather than a Tcl list, the body's scope, what
each iteration yields, and that a zero-iteration run preserves the
binder:

```tcl
command foreach_in_collection {
    dialects all-tcl
    traits {CONTROL_FLOW HAS_LOOP_BODY NEVER_INLINE_BODY LOOP_LIST_HEADER}
    arity 3
    required_package sdc

    arg 0 -role VarWrite
    arg 2 -role Body

    analyser_hook -native Foreach

    semantics {
        iterate {
            binder -arg 0 -grammar vendor.single_variable
            iterable -arg 1 -kind vendor.collection
            body -arg 2 -scope enclosing
            yield -semantic vendor.object_handle
            cardinality -from vendor.collection_summary
            completion -contract vendor.collection_loop_completion
            zero_iterations -bindings preserve
        }
    }
    evaluate none
}
```

`append_to_collection` and `remove_from_collection` declare one target
that the call may write, and no evaluator — a vendor collection handle
gives nothing to fold:

```tcl
command append_to_collection {
    dialects all-tcl
    arity 2..
    required_package sdc

    arg 0 -role VarWrite

    semantics {
        stores -targets {0} -outcome may_write
    }
    evaluate none
}
```

The `may_write` outcome makes the target a conditional write, so the store
ahead of a call is live: under `--dialect synopsys-eda-tcl`,
`set c {}; append_to_collection c $objs; return $c` in a procedure draws
no W220, and O109 keeps `set c {}`.

### A private command in a workspace pack

`tenant::label` is the completion-test fixture
(`rust/tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec`, under
`speclib tenant 2.2`), reproduced verbatim. The fixture declares a renamed
twin (`tenant::tag`) and a subcommand form (`tenant label NAME`)
identically, and none of the three needs a consumer edit:

```tcl
command tenant::label {
    arity 1
    semantics {
        effects {no_store_writes no_external_io}
        result -semantic string
    }
    evaluate -implementation tenant.label.v1 -host bounded_tcl {
        inputs {arg 0 exact}
        depends {tcl_profile implementation_identity}
        budget {-commands 2000 -wall-clock 20 -value-bytes 65536}
        body {name} { fold [string cat "tenant:" $name] }
    }
    facts {
        result -string_segments {{constant "tenant:"} {operand 0}}
        taint  -result_from {arg 0}
    }
}
```

`tenant::label acme` folds to `tenant:acme` on the implementation route
under 8.6, 9.0, and 9.1; under 8.4, 8.5, and `f5-irules` it declines
`unsupported`, because the body's `string cat` is not a command those
releases have, and a declared implementation runs under the analysed
release rather than answer for one it cannot run in. With an argument the
lattice does not know exactly the implementation is not run
(`declined: not-exact`), and the result is unknown; the `facts` block is
carried with the declaration and read by no solver. The `depends` row
names what an answer rests on: the analysed release and the
implementation's identity.

## Related docs

- [value-transfers.md](value-transfers.md) — the consumer interface contract these programs exercise
- [value-evaluation.md](value-evaluation.md) — the routes the declarations name
- [value-transfers-migration.md](value-transfers-migration.md) — the inventory and the tables these examples index
- [diagnostic-policy.md](diagnostic-policy.md) — the one findings pipeline the O111 and W100 rows answer to
- [registry-consumer-contracts.md](registry-consumer-contracts.md) — the other axes these programs touch and do not own
- [interprocedural-call-site-seeding.md](interprocedural-call-site-seeding.md) — the call-site seed the proc-level summaries rest on
- [../spec-dsl-examples/README.md](../spec-dsl-examples/README.md), [../spec-dsl-examples/string.tclspec](../spec-dsl-examples/string.tclspec), [../spec-dsl-examples/subst.tclspec](../spec-dsl-examples/subst.tclspec) — the authorable spellings
- [compiler design index](README.md), [design docs index](../README.md)
