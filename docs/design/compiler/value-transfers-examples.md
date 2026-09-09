# Value transfers — worked examples

One program per optimisation and diagnostic code the value-axis design
touches, with what the tool reports today and what the contracts in
[value-transfers.md](value-transfers.md) and
[value-evaluation.md](value-evaluation.md) change, followed by the
declarations behind the examples as they would be written in the Rust
command registry and in a `.tclspec` pack. The four programs at the top of
the interface contract are the shortest of these; this page is the full
set, and its programs are the fixed witnesses the slices in
[value-transfers-migration.md](value-transfers-migration.md) keep.

> **Status.** Every "today" line below is an observation, not an
> assertion: each program was run through `tcl diag FILE --json` and
> `tcl opt FILE --profile full` (a single optimiser pass) built from `rust`
> at `f02f327`, with `--dialect irules`, `tcl8.5`, or `tcl8.4` where the
> example says so, and through Tcl 9.0.4 for ground truth. Every "under the
> contracts" line is the proposal. The declarations in the last section are
> proposed spellings, not loader syntax.

## How to read an example

Each code has up to three programs:

- **fires today** — the baseline the tool already reports, so the
  migration's byte-identity gate has something to pin;
- **would gain** — the same shape with a computed constant the shared
  lattice cannot see today (an `append`-built string, a `lappend`-built
  list, a `string range` result, a variable holding an option name), and
  what the contracts make of it;
- **must stay sound** — a rewrite or finding that a naive transfer would
  get wrong, and the rule that prevents it.

Where the optimiser and a diagnostic disagree today on the same program,
the example says so: that disagreement is the four-evaluator problem the
interface contract removes.

## What running the corpus found

Running the programs exposed eight defects in today's tree. Each is an
issue, and each is a contract point:

| Issue | Program | Contract point |
|---|---|---|
| [#2050](https://github.com/bitwisecook/tcl-lsp/issues/2050) | O109 deletes `set n 1` before `set result [incr n]`; the program's output changes | a cell update's read is an SSA use by construction |
| [#2051](https://github.com/bitwisecook/tcl-lsp/issues/2051) | W220 and O109 delete `set a before` ahead of a `regexp … a b` that does not match; the optimised program errors | *preserve* is a storage outcome |
| [#2052](https://github.com/bitwisecook/tcl-lsp/issues/2052) | `set p { again}; append s $p` is rewritten to `append s again` | exact-value ingress: `parse_literal_value` trims |
| [#2053](https://github.com/bitwisecook/tcl-lsp/issues/2053) | deleting `set p "again"` leaves a `"` behind | rewrites are validated against the source they edit |
| [#2054](https://github.com/bitwisecook/tcl-lsp/issues/2054) | W231 reports "list has 0 elements" after `lappend xs a b c` | the length after a cell update is a fact, not the last literal `set` |
| [#2055](https://github.com/bitwisecook/tcl-lsp/issues/2055) | IRULE3101 flags `set p /a; HTTP::path $p` | a diagnostic reads the same value the optimiser forwards |
| [#2056](https://github.com/bitwisecook/tcl-lsp/issues/2056) | IRULE1201 counts an `HTTP::respond` in a branch I230 proves dead | applied reachability is one fact with many consumers |
| [#2057](https://github.com/bitwisecook/tcl-lsp/issues/2057) | W242 warns about a `while` loop O112 removes as never running | W240–W242 consume the branch fact instead of the condition's text |

## Optimisations

### O100 · propagate constant variables

```tcl
set retries 1
incr retries
puts "$retries"            ;# today: O100 inlines 2 — the incr arm is the one cell transfer SCCP has
```

```tcl
set acc ""
append acc foo
append acc bar
puts $acc                  ;# today: O104 folds the chain to `set acc foobar`; the read stays `$acc`
```

Today the chain folds textually (O104) but `acc` is `Overdefined` in the
lattice, so nothing forwards `foobar` into the read in the same pass. Under
the contracts the cell update makes `acc` `Const("foobar")` at the read;
O100 forwards it, and the code stays O100 rather than O102 because the
defining statement is a computed write.

```tcl
set n 1
set result [incr n]        ;# today: O109 deletes `set n 1` (#2050); the optimised program prints 1 and 1
puts $result
puts $n
```

Under the contracts the nested `incr` reads `n` as an SSA use, so the
store stays; the result `2` may be forwarded into `puts $result` while the
`incr` itself is preserved (permission 2 of the three permissions).

### O101 · fold constant integer expressions

```tcl
set x [expr {2 + 3}]       ;# today: O101 rewrites to `set x 5`
```

```tcl
set s foo
append s bar
set n [expr {[string length $s] * 2}]   ;# today: nothing — the reached [string length …] is declined and s is unknown
set t [expr {"x"}]                      ;# today: nothing — the adapter's result type is numeric only
set z [expr {0 && [error never]}]       ;# today: z is proven 0 — the evaluator already short-circuits
```

The evaluator already stops at `0 &&`, so the substitution on the right is
never reached; that laziness is kept. Under the contracts
`[string length $s]` is resolved through the registry's semantics when it
is reached, `"x"` folds because the boundary carries the engine's full
value, and `n` becomes `6`.

### O102 · forward a single reaching literal load

```tcl
set n 7
puts $n                    ;# today: O102 forwards 7 and O109 removes the store
```

Unchanged: a literal load is a literal load. A computed write forwards as
O100, never as O102.

### O103 · fold static procedure calls

```tcl
proc double {n} { expr {$n * 2} }
set x [double 21]          ;# today: O103 folds the call to 42
```

```tcl
proc label {} { set s abc; append s def; return $s }
set x [label]              ;# today: O104 folds the body's chain to `set s abcdef`; [label] is not folded
```

Under the contracts the argument-sensitive path re-runs SCCP on the callee
with the cell update present and folds `[label]` to `abcdef`; the
argument-independent summary path needs `summarise_returns` over a seedless
lattice (slice 7).

### O104 · fold string build chains

```tcl
set s "hello"
append s " world"          ;# today: O104 folds to `set s {hello world}`; an unrelated statement between the writes is tolerated
```

```tcl
set s hello
set p again
append s $p                ;# today: O102 forwards `again` into the call; the chain is not folded in the same pass
puts $s
```

Under the contracts the classifier dispatches on the resolved cell update
instead of three command names, and the chain folds through the lattice
value at the last write, `helloagain`. With `set p { again}` today's
rewrite drops the leading space (#2052); the exact-value ingress keeps it.

### O105 · redundant computation (GVN/CSE)

```tcl
llength $items
llength $items             ;# the shape the pass documents: top level, a CSE_CANDIDATE command, plain-word arguments
```

For the shapes tried at this revision — a literal list (both calls fold as
O129 first), and a list from `$argv` or `[gets stdin]` under `full` and
`aggressive` — the CLI reported nothing. Unchanged by the design beyond
reachability; the contract rule stands that the same value is not the same
observable computation.

### O106 · hoist loop-invariant computations

```tcl
foreach i $list {
    set n [llength $list]  ;# the shape the pass documents: a top-level loop, an invariant pure command
    puts "$i $n"
}
```

As for O105, no report was observed for the shapes tried. Unchanged:
hoisting must not introduce execution on the zero-trip path or move an
error.

### O107 · eliminate unreachable code

```tcl
proc p {x} {
    return $x
    puts "never reached"   ;# today: O107
}
```

```tcl
set x b
switch $x {
    a { puts A }           ;# today: no O107 and no I231 — the CFG subject is a Raw operand
    b { puts B }
    default { puts D }
}                          ;# today: O112 eliminates the whole switch from its own Env
```

Under the contracts step 1 of the `switch` plan resolves the whole-variable
`Raw` operand, the dead arm bodies leave `executable_blocks`, and O107 and
I231 fire; O112 consumes the same selection fact instead of its private
`Env`.

### O108 · eliminate transitively dead code

```tcl
proc report {x} {
    set a 1                ;# today: O108 — dead once b is removed
    set b [expr {$a + 1}]  ;# today: O126 — never read
    return $x
}
```

```tcl
proc p {} {
    set unused "\{"
    lappend unused value   ;# today: kept — `lappend` is never deletable; it raises "unmatched open brace in list"
    return 0
}
```

The second program stays as it is: a dead target with pure argument words
does not make the command removable. Under the contracts removal needs the
totality proof of permission 3: old value well-formed for the operation,
place proven bound, no trace.

### O109 · eliminate dead stores

```tcl
set limit 1                ;# today: O109 and W220
set limit 2
puts $limit
```

Two rewrites this pass makes today are unsound and are the witnesses for
the storage contract: the store read by a nested `[incr n]` (#2050) and
the store ahead of a `regexp` that does not match (#2051). A third is a
span defect: deleting `set p "again"` leaves the closing quote behind
(#2053).

### O110 · canonicalise expressions

```tcl
proc p {x} {
    set y [expr {$x + 1 + 2}]   ;# today: O110 rewrites to `$x + 3`
    return $y
}
```

```tcl
set x 10000000000000000.0
expr {$x + 1 + 2}          ;# 10000000000000002.0
expr {$x + 3}              ;# 10000000000000004.0
```

With `x` known the whole expression folds correctly today; with `x` unknown
the reassociation changes the result for doubles. Under the contracts
reassociation consumes the type and target proofs, and the floating-point
pair above is a fixed regression.

### O111 · brace-expression hints

```tcl
set x 1
set y [expr $x + 1]        ;# today: W100; O111 is never emitted by the CLI
```

O111 is produced in the language server from the already-lifted W100
(`append_brace_expr_perf_hints`), so it depends on W100 surviving
presentation. Under the contracts both consume the unbraced-expression
fact, or the product encodes one rule-group policy.

### O112 · eliminate constant-condition compounds

```tcl
if {1} { puts yes } else { puts no }   ;# today: O112 and I230
```

```tcl
set acc ""
append acc foo
if {$acc eq "foo"} { puts yes } else { puts no }   ;# today: nothing beyond the O104 chain fold
```

Under the contracts the cell update decides the condition; O112 consumes
the selection fact, and its first-unfoldable-clause and `catch`-descent
limits go.

### O113 · strength reduction

```tcl
proc p {key} {
    if {$key % 8} { return spill }   ;# today: O113 rewrites to `$key & 7`
    return [expr {$key ** 2}]
}
```

Unchanged; integer versus float and the target's arithmetic rules stay
explicit, and Tcl identities are never transferred to BPF-Tcl.

### O114 · the `incr` idiom

```tcl
set count 0
set count [expr {$count + 1}]   ;# today: O114 rewrites to `incr count`
```

Unchanged; the rewrite's error, initial-existence, and trace behaviour must
agree with `incr`'s.

### O115 · redundant nested `expr`

```tcl
proc p {x} {
    if {[expr {$x > 0}]} { return pos }   ;# today: O115 and W114
    return neg
}
```

Unchanged.

### O116 · fold a constant `list`

```tcl
puts [list a b c]          ;# today: O116 folds to {a b c}
```

```tcl
set a x
append a y
puts [list $a b]           ;# today: not folded — a is unknown to the lattice
```

Under the contracts the direct route folds it to `{xy b}` from the cell
update's value, with exact serialisation from the shared list owner.

### O117 · `[string length $s] == 0`

```tcl
proc p {s} {
    if {[string length $s] == 0} { return empty }   ;# today: O117 rewrites to `$s eq ""`
    return full
}
```

Unchanged.

### O118 · fold a constant `lindex`

```tcl
set val [lindex {red green blue} 1]   ;# today: O118 folds to green
```

```tcl
set l {}
lappend l red
lappend l green
set val [lindex $l 1]      ;# today: O130 folds the chain to `set l {red green}`; the lindex is not folded in the same pass
```

Under the contracts the cell update gives `l` its value at the read and
the direct route folds `green`.

### O119 · pack consecutive `set` literals

```tcl
proc p {} {
    set a 1
    set b 2
    set c 3                ;# today: O119 packs into `lassign {1 2 3} a b c`
    return [list $a $b $c]
}
```

Unchanged; the packed form must agree on results, traces, partial failure,
and scope, not only on final values.

### O120 · `eq`/`ne` for string comparison

```tcl
proc p {name} {
    if {$name == "admin"} { return 1 }   ;# today: O120 and W110
    return 0
}
```

Unchanged; W110 is not an edit certificate.

### O121, O122, O123 · recursion

```tcl
proc fact {n acc} {
  if {$n <= 1} { return $acc }
  return [fact [expr {$n-1}] [expr {$n*$acc}]]   ;# today: O121 rewrites to tailcall
}
proc countdown {n} {
    if {$n <= 1} { return 1 } else { countdown [expr {$n - 1}] }   ;# today: O122 converts to a while loop
}
proc fact2 {n} {
    if {$n <= 1} { return 1 } else { return [expr {$n * [fact2 [expr {$n - 1}]]}] }   ;# today: O123, hint only
}
```

Unchanged.

### O124 · unused iRule procs

```tcl
proc legacy {} { return 1 }   ;# today (--dialect irules): O124
when HTTP_REQUEST { pool main }
```

Unchanged; a reachability-aware call graph is a follow-on.

### O125 · sink assignments into a decision block

```tcl
proc p {ok} {
    set msg "error"        ;# today: O125 sinks msg into the else branch
    if {$ok} { return } else { puts $msg }
}
```

Unchanged; a cell update is sinkable under the same rule, optionally.

### O126 · unused assignments

```tcl
proc handle {} {
    set unused 42          ;# today: O126 and W211
    puts done
}
```

Unchanged; the unused fact is separate from the producer's effects.

### O127 · inline a single-use assignment

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]    ;# today (--dialect irules): O127 inlines into `log local0. [set uri [HTTP::uri]]`
    log local0. $uri
}
```

Unchanged; a constant use takes O100 instead.

### O128 · end-offset indices

```tcl
proc p {L s} {
    set last [lindex $L [expr {[llength $L] - 1}]]              ;# today: O128 → end
    set last_char [string index $s [expr {[string length $s] - 1}]]   ;# today: O128 → end
    return "$last $last_char"
}
```

Unchanged here; its length-position table is `ArgRole::Index` plus
`ReturnElements` debt on another axis.

### O129 · fold a pure builtin substitution

```tcl
set s [string range foobarbaz 3 6]   ;# today: O129 folds to barb
```

```tcl
set h [binary format H* 00ff]        ;# today: nothing — pure, no evaluator
puts [string length $h]
```

Under the contracts `binary format` takes the direct route through
`tcl_cmd_core::binary::format`, the value carries byte-array
representation evidence, `string length` folds to `2`, and the
non-printable value is never spliced into source. The rule that a known
result never replaces a writing producer keeps `[incr n]` out of O129.

### O130 · fold `lappend` chains

```tcl
set l {}
lappend l a b              ;# today: O130 folds to `set l {a b}`
```

```tcl
lappend l a                ;# today: nothing — the chain needs a literal initial set
lappend l b
puts $l
```

Under the contracts the initial cell's existence is a fact (absent, so
`lappend` creates it), and the chain folds from `lappend l a`.

## Diagnostics

### I230 · constant branch condition

```tcl
set x 1
if {$x == 1} { puts one } else { puts other }   ;# today: I230
```

```tcl
set acc ""
append acc foo
if {$acc eq "foo"} { puts yes } else { puts no }            ;# today: nothing
set s abcdef
if {[string length $s] == 6} { puts six } else { puts other } ;# today: nothing
```

Under the contracts both decide: the first through the cell update, the
second through the lazily resolved command substitution in the condition.

### I231 · constant switch arm

```tcl
switch -- 1 {
    1       { puts "one" }
    2       { puts "two" }     ;# today: I231
    default { puts "other" }
}
```

```tcl
set x b
switch $x { a { puts A } b { puts B } default { puts D } }   ;# today: O112 fires; no I231
switch -glob -- $x { a* { puts A } b* { puts B } }           ;# today: O112 fires; no I231
```

Under the contracts the whole-variable `Raw` case decides in the CFG, and
the opaque `-glob` form gets a selection fact from `tcl_cmd_core::switch`
that I231, O112, and the analyser consume together.

### W124 · invalid IP literal

```tcl
set addr "10.0.0.256"      ;# today: W124 — every Const(String) in the lattice is checked
puts $addr
```

```tcl
set addr 10.0.0
append addr .256           ;# today: nothing
puts $addr
```

Under the contracts the computed string is a lattice constant and W124
sees it, anchored by the literal-substring rule with whole-statement
fallback.

### W121 · non-contiguous subnet mask

```tcl
when CLIENT_ACCEPTED {
  set m 255.0.255.0        ;# today (--dialect irules): W121 at the set and at the use — a Const(String), like W124
  if {[IP::addr [IP::client_addr] mask $m] eq "10.0.0.0"} { reject }
}
```

```tcl
when CLIENT_ACCEPTED {
  set m 255.0
  append m .255.0          ;# today: nothing
  if {[IP::addr [IP::client_addr] mask $m] eq "10.0.0.0"} { reject }
}
```

### W233 · division by a provably zero divisor

```tcl
proc p {x} { return [expr {$x / 0}] }   ;# today: W233
```

```tcl
proc p {x} {
    set z [string range 1000 3 3]
    return [expr {$x / $z}]    ;# today: no W233 — the optimiser folds z to 0, the diagnostic lattice does not
}
```

### W230, W231, W232 · constant index out of range

```tcl
set tail [lindex {a b c} end-5]    ;# today: W230
set xs {a b c}
lset xs 5 X                        ;# today: W231
set last [string index "abc" end-5] ;# today: W232
```

```tcl
set l {a b c}
set tail [lindex $l 9]     ;# today: no W230 — while the optimiser folds it to {} (O118)
set s abc
set last [string index $s 9]   ;# today: no W232 — while the optimiser folds it to {} (O129)
```

```tcl
set xs {}
lappend xs a b c
lset xs 2 X                ;# today: W231 "list has 0 elements" (#2054); the program prints `a b X`
```

Under the contracts the container length comes from the value after the
cell update, and the syntactic half of W230/W232 reads the lattice.

### W210 · read before set

```tcl
proc p {} {
    regexp xy zz m         ;# today: W210 — the private prover decides a literal pattern without metacharacters
    puts $m
}
```

```tcl
proc p {} {
    regexp {(x)(y)} zz a b ;# today: nothing — the prover bails on metacharacters
    puts "$a $b"
}
```

```tcl
proc p {} {
    set a before
    set b before
    regexp {(x)(y)} zz a b ;# today: W220 on both sets; O109 deletes them; the program then errors (#2051)
    puts "$a $b"
}
```

Under the contracts the regexp owner evaluates the match through our
engine; the no-match outcome *preserves* `a` and `b`; W210 consumes the
existence proof for the first two programs, and the third keeps its stores.

### W211, W213, W214, W220, H300 · variable lifecycle

```tcl
proc p {} {
    set x 1                ;# today: no W211 — the read in the dead arm counts as a use
    if {0} { puts $x }     ;# today: I230, and O112 removes the arm
}
proc q {} { unset maybe_defined }         ;# today: W213
proc greet {name greeting} { puts "Hello, $name" }   ;# today: W214 on greeting
set y 1                    ;# today: W220
set y 2
puts $y
set z 1                    ;# today: H300
set z 1
```

Under the contracts W211 and W214 read applied reachability, unbind
outcomes give W213 read-after-`unset` precision later, and W220's false
positive on a preserved target (#2051) goes.

### W126 · non-channel value in channel position

```tcl
puts "output.txt" "hello"             ;# today: W126
set ch [string cat out put.txt]
puts $ch "hello"                      ;# today: W126 — the type lattice knows `string cat` returns a string
```

Unchanged in effect: the semantic type travels with the value.

### W123, W307, W308 · dispatch heads

```tcl
unknownCmd 1               ;# today: W123
set cmd unknown
append cmd Cmd
$cmd 1                     ;# today: nothing — the head is unknown, so neither W123 nor W307 fires
```

```tcl
set cmd [gets stdin]
$cmd hello                 ;# today: W307 and T100
set cmd puts
$cmd hello                 ;# today: no W307 — the proven literal suppresses it; O102 forwards `puts`
set cmd pu
append cmd ts
$cmd hello                 ;# today: W307 — the computed head is unknown
```

```tcl
oo::class create Point { method x {} { return 0 } }
set p [Point new]
$p distance                ;# today: W308
set m dist
append m ance
$p $m                      ;# today: nothing
```

Under the contracts the computed heads resolve: `unknownCmd` gets W123,
`puts` suppresses W307, `distance` gets W308, and a computed head is never
rename-safe.

### S100, S101, S102, S103, S110 · representation

```tcl
set v 5
expr {$v + 1}
lindex $v 0                ;# today: S100 — numeric intrep read as a list
set d [dict create a 1]
puts [lindex $d 0]         ;# today: S100 — dict intrep read as a list
set items [list 1 2 3]
foreach item $items { puts [expr {$item + 0}]; puts [lindex $item 0] }   ;# today: S101
set total 0
foreach item {1 2 3} { set total [expr {$total + 1}]; set total [string range $total 0 end] }   ;# today: S101 and S102
set a [lrepeat 1000 x]
set b $a
lappend b y                ;# today: S103
set bin [binary format H* ff00]
set u [string toupper $bin] ;# today: S110 — the registry's return type marks the byte array
```

Under the contracts the exact value, the semantic type, and the
representation evidence are three facts: once `binary format` folds, S110
still fires from the representation evidence rather than from a lattice
string, and a computed constant never manufactures or hides a conversion.

### T100–T106, IRULE3001–3004, W313 · taint

```tcl
set cmd [gets stdin]
eval $cmd                  ;# today: T100
set name [gets stdin]
puts $name                 ;# today: T101
set pattern [gets stdin]
set matches [glob $pattern]   ;# today: T102 (and W304)
set pat [gets stdin]
regexp $pat abc            ;# today: T103
set host [gets stdin]
socket $host 80            ;# today: T104
set child [interp create -safe]
interp eval $child $cmd    ;# today: T105
proc p {userPath} { file delete $userPath }   ;# today: W313
```

```tcl
set cmd [gets stdin]
if {0} { eval $cmd }       ;# today: I230, no T100 — taint already reads applied reachability
```

```tcl
when HTTP_REQUEST {
  set host [HTTP::host]
  HTTP::respond 200 content "<h1>$host</h1>"   ;# today: IRULE3001
  log local0. "Host: $host"                    ;# today: IRULE3003
}
when HTTP_RESPONSE {
  set val [HTTP::header value X-Custom]
  HTTP::header replace X-Reply $val            ;# today: IRULE3002
}
when HTTP_REQUEST {
  set target [HTTP::header value Location]
  HTTP::redirect $target                       ;# today: IRULE3004
}
```

```tcl
when HTTP_REQUEST {
  set u [URI::encode [HTTP::uri]]
  set v [URI::encode $u]     ;# today: T106 — $u already carries the encoder's colour
  HTTP::redirect "https://example.com/?next=$v"
}
```

Unchanged: taint never reads values, colour flows through write outcomes,
and only applied reachability can drop a flow.

### IRULE3101, IRULE3102, IRULE3103 · URI setters and getters

```tcl
when HTTP_REQUEST { HTTP::uri "newpath" }    ;# today: IRULE3101
when HTTP_REQUEST {
  set p /a
  HTTP::path $p            ;# today: IRULE3101 (#2055) — while O102 forwards /a into the same call
}
when HTTP_REQUEST {
  if {[string match "*admin*" [HTTP::path]]} { reject }   ;# today: IRULE3102
}
when HTTP_REQUEST {
  set uri [HTTP::uri]
  set parts [split $uri "?"]   ;# today: IRULE3103
  if {[lindex $parts 0] eq "/admin"} { reject }
}
```

Under the contracts the setter check reads the lattice constant, so a
subject proven to start with `/` is clean.

### IRULE1005–1008, 1201, 1202, 4002, 4004, 5002, 5004 · protocol and events

```tcl
when HTTP_REQUEST_DATA { log local0. [HTTP::payload] }   ;# today: IRULE1005
when HTTP_REQUEST { set p [HTTP::payload] }              ;# today: IRULE1006
when CLIENT_ACCEPTED { TCP::collect 1024 }               ;# today: IRULE1007
when CLIENT_ACCEPTED { TCP::release }                    ;# today: IRULE1008
when HTTP_REQUEST { HTTP::respond 200; HTTP::header insert X-Custom val }   ;# today: IRULE1201
when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/blocked"} { HTTP::respond 403 }
  HTTP::redirect "https://example.com/"                  ;# today: IRULE1202
}
when RULE_INIT { set static::debug 0 }                   ;# today: IRULE4002
when HTTP_REQUEST { set pool_name "main_pool" }          ;# today: IRULE4004
when HTTP_REQUEST { drop; log local0. "after drop" }     ;# today: IRULE5002
when DNS_REQUEST { DNS::return "1.2.3.4"; log local0. "still runs" }   ;# today: IRULE5004
```

```tcl
when HTTP_REQUEST {
  if {0} { HTTP::respond 200 }
  HTTP::header insert X-Custom val   ;# today: IRULE1201 although I230 proves the arm dead (#2056)
}
when HTTP_REQUEST {
  set pool_name [string range main_pool_v2 0 8]   ;# today: no IRULE4004 — the value contains `[`
}
```

Under the contracts the response-commit walk consumes applied
reachability, and IRULE4004 sees a computed constant as hoistable.

### W201 · manual path concatenation

```tcl
set dir /tmp
set filename report.txt
set path "$dir/$filename"  ;# today: W201
```

Unchanged in effect: rendered-value properties get exact flags from
computed values.

### W303 · ReDoS pattern

```tcl
proc p {input} { regexp {(a+)+$} $input }   ;# today: W303
```

```tcl
proc p {input} {
    set re {(a+)+$}
    regexp $re $input      ;# today: no W303 — while O100 propagates the pattern into the call
}
```

Under the contracts the pattern-role check reads the lattice constant.

### W240, W241, W242 · loop conditions

```tcl
while {0} { puts "never runs" }   ;# today: W240 and I230
while {1} { puts "forever" }      ;# today: W241
proc p {} {
    set i 0
    while {$i < 10} { puts $i }   ;# today: W242
}
```

```tcl
set n 0
while {$n} { puts "never runs" }  ;# today: I230, O112 removes the loop, W242 — no W240 (#2057)
set go 1
while {$go} { puts "forever" }    ;# today: W242 and O101 folds the condition — no W241
```

Under the contracts W240 and W241 consume the branch fact I230 uses.

### W127, W137, W138, W141, W145, W146, W147, W152, W200 · literal-only option and value checks

```tcl
when HTTP_RESPONSE priority 5 { HTTP::version "2.0" }   ;# today (irules): W127
if {[string is dict $config]} { puts dict }              ;# today (tcl8.5): W137
puts [format "flags: %b" 5]                              ;# today (tcl8.5): W138
proc r {path} { return -code error -errorstack {CALL load extra} "cannot read $path" }   ;# today: W141
puts [string l $s]                                       ;# today: W145
trace add variable ::config(port) {read rename write} logChange   ;# today: W146
set l [lsort -increasing -decreasing {b a}]              ;# today: W147
::bibtex::parse -command handle -recordcommand rec       ;# today: W152 (and W147)
set data [binary format iu 42]                           ;# today (tcl8.4): W200
```

```tcl
when HTTP_RESPONSE priority 5 { set v 2.0; HTTP::version $v }   ;# today: nothing — while O102 forwards 2.0
set cls dict; if {[string is $cls $config]} { puts dict }       ;# today (tcl8.5): nothing
set f "flags: %b"; puts [format $f 5]                           ;# today (tcl8.5): nothing
proc r {path} { set es {CALL load extra}; return -code error -errorstack $es "cannot read $path" }   ;# today: nothing
set sub l; puts [string $sub $s]                                ;# today: nothing
set ops {read rename write}; trace add variable ::config(port) $ops logChange   ;# today: nothing — while O100 propagates the list
set o -decreasing; set l [lsort -increasing $o {b a}]           ;# today: nothing
set fmt iu; set data [binary format $fmt 42]                    ;# today (tcl8.4): nothing — while O102 forwards iu
```

Under the contracts each check feeds the exact lattice value through its
existing validator (`LiteralArgumentValidator` for W146) with honest
provenance; the source token is never relabelled a literal.

## The declarations behind the examples

Every "today" excerpt below is the shipped Rust spec as it is in the tree;
every "proposed" block is a sketch of the spelling the contracts call for,
in Rust and in `.tclspec`, and none of it is loader syntax yet.

### `incr`, `append`, `lappend`: a cell update

Today, `rust/tcl-registry/src/commands/tcl/incr_.rs`:

```rust
CommandSpec {
    name: "incr",
    arity: Arity::new(1, 2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    assigns_variable_at: Some(0),
    safe_on_uninit: Some(SpecSurface::TCL85_PLUS),
    return_type: Some(TclType::Int),
    lowering_hook: Some(LoweringHookId::Incr),
    native_lowering: Some(NativeLowering::CellReadModifyWrite(CellUpdate::Increment)),
    inline_codegen_hook: Some(InlineCodegenHookId::Incr),
    analyser_hook: Some(AnalyserHookId::Incr),
    ..CommandSpec::DEFAULT
}
```

`append_.rs` and `lappend_.rs` carry the same shape with
`CellUpdate::Append` and `CellUpdate::ListAppend`. The operation is
already declared; what is missing is the specialisation that turns it into
a value transfer.

Proposed, in Rust:

```rust,ignore
CommandSpec {
    name: "incr",
    // every existing field unchanged …
    semantics: registry_semantics!(incr::SEMANTICS),
    ..CommandSpec::DEFAULT
}

mod incr {
    pub struct Semantics;
    pub const SEMANTICS: &dyn CommandSemantics = &Semantics;

    impl CommandSemantics for Semantics {
        /// Derived from `CellReadModifyWrite(Increment)`: one place, read
        /// then written; the possible failure is the operation's.
        fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
            cell_update_plan(input, CellUpdate::Increment)
        }
        fn transfer(&self, domain: FactDomain, input: &dyn AnalysisInputs) -> TransferAnswer {
            match domain {
                FactDomain::Type => type_answer(TclType::Int, per_target![TclType::Int]),
                FactDomain::Range => range::integer_add(input),   // the abstract model, not the evaluator
                _ => TransferAnswer::Generic,
            }
        }
        fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
            let form = incr_form(input.invocation())?;
            let target = form.declared_target();
            let old = input.prior_store(target.place(), FactDomain::ExactValue);
            let step = form.increment_or_exact_default("1");
            // The shared numeric owner under the profile: 8.4 declines past
            // the wide boundary, 8.5+ widens, `010` differs by release.
            let out = numeric_core::tcl_incr(old, step, input.context(), budget)?;
            EvalAnswer::Evaluated(InvocationOutcome {
                completion: CompletionOutcome::Normal,
                result: out.value.clone().into(),
                ordered_stores: vec![StoreOutcome::Write { target, value: out.value }],
                types: TypeFacts::int(),
                evidence: input.context().binding_evidence(),
            })
        }
    }
}
```

Proposed, in `.tclspec`, for the shipped command and for a pack command
that reuses the same operation by name:

```tcl
command incr {
    arity 1..2
    arg 0 -role VarWrite
    return_type Int
    semantics -native core.incr        ;# the read-modify-write plan
    evaluate  -direct core.incr        ;# the shared numeric owner
    facts     -native core.incr        ;# Int result and Int target
}

command counter::bump {
    arity 1..2
    arg 0 -role VarWrite
    traits {READS_BEFORE_WRITE FIRST_ARG_VARNAME}
    return_type Int
    semantics -native core.incr        ;# same operation, same evaluator: no body runs
    evaluate  -direct core.incr
}
```

### `string range`, `binary format`: a pure result on the direct route

Today, the `range` subcommand in `string_.rs` declares `pure: true`,
`return_type: Some(TclType::String)`, and `const_fold: Some(fold_range)`,
an ASCII-only re-implementation beside `tcl_cmd_core::string::range`;
`binary format` declares `pure: true` and no evaluator. In a pack, today's
authorable form (`docs/design/spec-dsl-examples/string.tclspec`) is a
`const_fold` body that calls the real command in the sandbox:

```tcl
subcommand range {
    arity 3
    pure
    return_type String
    const_fold {words ctx} {
        if {[llength $words] != 3} return
        lassign $words s first last
        if {![string is ascii $s]} return
        fold [string range $s $first $last]
    }
}
```

Proposed, in Rust:

```rust,ignore
fn evaluate(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let [s, first, last] = exact_operands(input, [0, 1, 2])?;
    // Admissibility before the core: the character model and the index
    // numeral grammar must be unanimous when the profile names no release.
    let mut ops = ConstOps::admit(input.context(), Needs::CHAR_MODEL | Needs::INDEX_GRAMMAR)?;
    let value = tcl_cmd_core::string::range(&mut ops, &s, &first, &last).map_err(decline)?;
    exact_result(ops.take(value), TypeFacts::string())
}

fn evaluate_binary_format(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let words = exact_operands_from(input, 0)?;
    let mut ops = ConstOps::admit(input.context(), Needs::BINARY_SIGNEDNESS)?;
    budget.charge_bytes(tcl_cmd_core::binary::format_size_bound(&words)?)?;   // bound before allocation
    let bytes = tcl_cmd_core::binary::format(&mut ops, &words).map_err(decline)?;
    exact_result(ops.take(bytes), TypeFacts::byte_array())               // representation evidence, not a string type
}
```

Proposed, in `.tclspec`:

```tcl
subcommand range {
    arity 3
    pure
    return_type String
    semantics -native core.string_range
    evaluate  -direct core.string_range
}

subcommand format {
    arity 1..
    pure
    return_type ByteArray
    semantics -native core.binary_format
    evaluate  -direct core.binary_format
    facts { result -representation byte_array }
}
```

### `expr`: the expression route

Today `expr_.rs` declares `arg_roles: &[(0, ArgRole::Expr)]`,
`return_type: Some(TclType::Numeric)`, `lowering_hook: Some(LoweringHookId::Expr)`,
and `native_lowering: Some(NativeLowering::Structured(LoweringHookId::Expr))`;
the evaluation lives in the compiler's `cmd == "expr"` arm and the numeric-only
`FoldOps` adapter.

Proposed, in Rust:

```rust,ignore
fn evaluate(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    // Braced argument: the expression text. Quoted or several arguments:
    // substitutions already performed, then concatenated, as the command specifies.
    let expression = expr_arguments::prepare(input.invocation())?;
    // Lazy services over the analysis inputs: a variable when reached, a
    // nested invocation through its registry semantics when reached and
    // effect-free, a math function only with binding evidence.
    let ops = ProvenTclExprOps::new(input, budget, NestedPolicy::EffectFreeOnly);
    shared_expr_engine::evaluate(expression, ops)   // the engine's full value, not to_number()
}
```

Proposed, in `.tclspec`:

```tcl
command expr {
    arity 1..
    arg 0 -role Expr
    semantics -native core.expr_arguments
    evaluate  -expression tcl.expr
    facts     -native core.expr_facts
}
```

### `regexp`, `scan`, `lassign`: several targets, write or preserve

Today `regexp_.rs` and `scan_.rs` resolve their `VarWrite` targets through
an `arg_role_resolver`, declare `var_write_typing: VarWriteTyping::Destructured`,
`return_type_hook: Some(ReturnTypeHookId::Regexp)` / `Scan`, and (`scan`)
`const_fold: Some(fold_scan)`; `lassign.rs` declares
`var_write_typing: VarWriteTyping::ElementsOf { container_arg: 0 }`. Each
target today is a definite definition, which is #2051.

Proposed, in Rust:

```rust,ignore
fn evaluate_regexp(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let form = regexp_form(input.invocation())?;           // flags resolved once
    let [pattern, subject] = exact_operands(input, form.pattern_and_subject())?;
    let mut ops = ConstOps::admit(input.context(), Needs::CHAR_MODEL)?;
    // The shared command core over our engine; the typed precision result
    // declines on fuel or depth exhaustion and on an approximate capture.
    match tcl_cmd_core::regex::regexp::<_, AreEngine>(&mut ops, &form.args(&pattern, &subject), budget)? {
        RegexpResult::Count { count, assign: Some(values) } => {
            let stores = form.targets().zip(values)
                .map(|(t, v)| StoreOutcome::Write { target: t, value: v }).collect();
            exact_outcome(count, stores, TypeFacts::int())
        }
        RegexpResult::Count { count, assign: None } => {   // no match: every target preserved
            let stores = form.targets().map(|t| StoreOutcome::Preserve { target: t }).collect();
            exact_outcome(count, stores, TypeFacts::int())
        }
        RegexpResult::Inline(list) => exact_result(list, TypeFacts::list()),
    }
}

fn evaluate_scan(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let [subject, format] = exact_operands(input, [0, 1])?;
    let parsed = tcl_cmd_core::scan::parse_format(&format)?;      // the format's own owner
    let (count, converted) = tcl_cmd_core::scan::convert(&subject, &parsed, input.context())?;
    // Converted targets are written in order; the rest are preserved (`scan {12 nope} {%d %d} a b`).
    let stores = scan_targets(input).enumerate().map(|(i, t)| match converted.get(i) {
        Some(v) => StoreOutcome::Write { target: t, value: v.clone() },
        None => StoreOutcome::Preserve { target: t },
    }).collect();
    exact_outcome(count, stores, TypeFacts::per_target(parsed.types()))   // %d and %s differ per target
}

fn evaluate_lassign(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let list = exact_operand(input, 0)?;
    let items = tcl_syntax::list::split_list(&list).map_err(decline)?;
    // Targets resolve to places first, so `lassign … a a` writes `a` twice in order.
    let targets: Vec<_> = lassign_targets(input).collect();
    let stores = targets.iter().enumerate().map(|(i, t)| StoreOutcome::Write {
        target: *t, value: items.get(i).cloned().unwrap_or_default(),   // a missing element binds ""
    }).collect();
    exact_outcome(join_list(&items[targets.len().min(items.len())..]), stores, TypeFacts::list())
}
```

Proposed, in `.tclspec`, for the shipped commands and for a pack command
with an authored evaluator that must say *preserve* itself:

```tcl
command regexp {
    semantics -native core.regexp_forms
    evaluate  -direct core.regexp
    facts     -native core.regexp_facts
}

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
        body {s} {
            set parts [split $s :]
            if {[llength $parts] != 3} {
                preserve 1; preserve 2; preserve 3   ;# silence would be a decline, not a preserve
                fold 0
            }
            lassign $parts a b c
            write 1 $a; write 2 $b; write 3 $c
            fold 3
        }
    }
}
```

### `switch`: selection semantics as data plus one declared contract

Today `switch_.rs` declares `case_list: Some(&CaseListSpec::SWITCH)` and
`lowering_hook: Some(LoweringHookId::Switch)`; `CaseListSpec::SWITCH`
carries `subject_args: 1`, the `-exact` / `-glob` / `-regexp` / `-nocase`
options, `fallthrough_body: Some("-")`, and the `-matchvar` / `-indexvar`
value options. The grammar is authorable today
(`docs/design/spec-dsl-examples/switch.tclspec` spells the same descriptor
out); the selection semantics are implemented three times in the compiler.

Proposed, in Rust:

```rust,ignore
impl CommandSemantics for SwitchSemantics {
    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        case_list_plan(input, &CaseListSpec::SWITCH)      // arms, bodies, fall-through, default: locations and grammar
    }
    fn transfer(&self, domain: FactDomain, input: &dyn AnalysisInputs) -> TransferAnswer {
        if domain != FactDomain::Selection { return TransferAnswer::Generic; }
        let subject = input.operand(SUBJECT, FactDomain::ExactValue);
        let options = tcl_cmd_core::switch::parse_options(&mut ConstOps::admit(input.context(), Needs::CHAR_MODEL)?, input.option_words())?;
        // One selected-edge fact per member of a finite subject, joined;
        // a pattern that cannot be evaluated keeps the error possibility.
        selection_fact(subject.members().map(|s| tcl_cmd_core::switch::select::<_, AreEngine, _>(&options, s, input.arms())))
    }
    fn evaluate(&self, _: &dyn AnalysisInputs, _: &mut Budget) -> EvalAnswer { EvalAnswer::Declined(DeclineReason::NotAValue) }
}
```

Proposed, in `.tclspec`, for a private dispatch-table command: the
grammar as data, and the execution contract named explicitly rather than
inferred from the grammar:

```tcl
command vendor::dispatch {
    arity 2..
    case_list {
        subject_args 1
        exact_option  -exact
        nocase_option -nocase
        end_options_option --
        fallthrough_body -
        keyword_patterns {default} -final-only
    }
    semantics -native core.switch_selection   ;# ordered first match, fall-through, final default
}
```

### `unset`, `dict with`: unbind, and a structural plan

Today `unset_.rs` resolves its targets through `unset_arg_roles` and
carries `Traits::DESTROYS_VARIABLE` (deliberately not
`assigns_variable_at`); `dict with` in `dict.rs` resolves
`VarWrite`, `VarRead`, and `Body` roles. The trait states the operation,
so the unbind outcome derives from it; the body command is a structural
plan, and the key binding a projection of a constant dict.

```rust,ignore
// unset: derived, no evaluator body
fn evaluate_unset(input: &dyn AnalysisInputs, _: &mut Budget) -> EvalAnswer {
    let stores = unset_targets(input).map(|t| StoreOutcome::Unbind { target: t }).collect();
    exact_outcome(ExactValue::empty(), stores, TypeFacts::string())   // `unset` of an unbound place is an error, not a value: the driver declines unless existence is proven
}

// dict with: bind, body, reconcile
fn structure_dict_with(input: &dyn AnalysisInputs) -> PlanAnswer {
    PlanAnswer::Body {
        binders: KeyBinding::from_dict_operand(input, DICT_ARG),   // the projection of a constant dict, when known
        body: BodyPlan::enclosing_scope(BODY_ARG),
        reconcile: Reconcile::WriteBackKeys(DICT_ARG),
        completion: CompletionProtocol::TclBody,
    }
}
```

```tcl
command unset {
    arity 0..
    traits {DESTROYS_VARIABLE}
    arg_role_resolver -native core.unset_roles
    semantics -native core.unset             ;# unbind outcomes, derived from the trait
}

subcommand with {
    arity 2..
    arg 0 -role VarWrite
    arg end -role Body
    semantics -native core.dict_with         ;# bind keys · body · write back
}
```

### A vendor loop and a private command in a workspace pack

Today `specs/sdc_base.tclspec` declares `foreach_in_collection` with
`traits {CONTROL_FLOW HAS_LOOP_BODY NEVER_INLINE_BODY LOOP_LIST_HEADER}`,
`arg 0 -role VarWrite`, `arg 2 -role Body`, and
`analyser_hook -native Foreach`, which lets the native handler treat a
braced literal iterable as a Tcl list.

Proposed:

```tcl
command foreach_in_collection {
    arity 3
    arg 0 -role VarWrite
    arg 2 -role Body
    semantics {
        iterate {
            binder    -arg 0 -grammar vendor.single_variable
            iterable  -arg 1 -kind vendor.collection       ;# a handle, never a Tcl list
            body      -arg 2 -scope enclosing
            yield     -semantic vendor.object_handle
            cardinality -from vendor.collection_summary
            completion -contract vendor.collection_loop_completion
            zero_iterations -bindings preserve
        }
    }
}

command tenant::label {
    arity 1
    semantics {
        effects {no_store_writes no_external_io}
        result -semantic string
    }
    evaluate -implementation tenant.label.v1 -host bounded_tcl {
        inputs {arg 0 exact}
        depends {tcl_profile implementation_identity}
        body {name} { return [string cat "tenant:" $name] }
    }
    facts {
        result -string_segments {{constant "tenant:"} {operand 0}}
        taint  -result_from {arg 0}
    }
}
```

With an unknown argument `tenant::label` is not invoked with a
placeholder: the answer is an unknown exact value plus the proven prefix
segment and the taint relationship. Under the rulings the pack's
declarations are authoritative once loaded; binding validity and the
implementation identity still decide when the model applies.

## Related docs

- [value-transfers.md](value-transfers.md) — the consumer interface contract these programs exercise
- [value-evaluation.md](value-evaluation.md) — the routes the declarations name
- [value-transfers-migration.md](value-transfers-migration.md) — the slices and the tables these examples index
- [registry-consumer-contracts.md](registry-consumer-contracts.md) — the other axes and the follow-ons
- [../spec-dsl-examples/README.md](../spec-dsl-examples/README.md), [../spec-dsl-examples/string.tclspec](../spec-dsl-examples/string.tclspec), [../spec-dsl-examples/switch.tclspec](../spec-dsl-examples/switch.tclspec) — today's authorable spellings
- [compiler design index](README.md), [design docs index](../README.md)
