# KCS: I230 — Why does the analyser say my `[info exists]` branch never runs?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, sccp, dataflow

## Profiles

default

## Question

Why does the analyser report that `[info exists X]` (or `[array exists X]`) is
"always false" — or "always true" — and that one branch of my `if` is
unreachable?

## Why

When a variable is provably set (or provably never set) at the point of the
check, `[info exists X]` has a known answer, so one arm of the `if` can never
run. The most common surprise is a **local** variable used for "remember this
between calls":

```tcl
proc authorize {} {
    if {[info exists handle]} {
        return $handle
    }
    return [ILX::init Access-Plugin Access-Extension]
}
```

Each call to `authorize` gets a **fresh** local scope — Tcl locals do not
survive from one invocation to the next. So `handle` is never set when the
check runs, `[info exists handle]` is always false, and the re-use branch is
dead. Re-entrancy does not change this: a new call (from APM, an ILX callback,
or anywhere) is a new frame with empty locals.

The fold reads a flow-sensitive fact: whether the variable is provably set,
provably unset, or uncertain at the exact point the check runs, tracking
every assignment and every `unset` on the way there — not merely whether the
name is ever written anywhere in the procedure. A `set handle …` earlier in
the body does not block the fold by itself:

```tcl
proc reset {} {
    set handle 1
    unset handle
    if {[info exists handle]} {   ;# I230: always false
        return $handle
    }
    return [ILX::init Access-Plugin Access-Extension]
}
```

`handle` is written and then removed before the check, so it is provably
unset there and the re-use branch is dead — even though the procedure
contains a `set handle`. A `set` that runs before the check with nothing
between them that undoes it makes the check certainly true, and `I230`
reports it as always true. What stops the fold is an assignment that
**reaches** the check on some paths and not others: `if {$c} {set handle
1}` leaves the check genuinely undecided, and no `I230` is reported.

The analyser folds the check to its constant value and reports **`I230`** on the
condition. The optimiser can then drop the dead branch
([`O107`](kcs-optimisation-o107-unreachable-dead-code.md)).

`I230` isn't only about `info exists` — it fires on **any** condition SCCP can
fold to a constant, including an ordinary expression on a proc parameter. One
source of that fact is interprocedural: when every call site to a proc passes
the same literal for a parameter, the analyser seeds that parameter as a
compile-time constant for the callee's own analysis. If your proc is
genuinely recursive (or mutually recursive) and a parameter varies with each
call — a counter, a depth, an accumulator — that variation is real evidence
against treating it as constant. A false `I230` here means the analyser
failed to see one of the varying call sites — most often because it was
inside a `namespace eval` block and reached via a bare recursive self-call,
because it was embedded inside a `catch { … }` / `uplevel { … }` body, or
because it was dispatched through a variable (`set cmd helper; $cmd dev`).
Report a reproducer if you see this: the fix is always to make
the call-site scan see the call, never to special-case the parameter. For
the full rule, see
[when a parameter is treated as a constant](../kcs-qa-when-is-a-proc-parameter-treated-as-a-constant.md).

## Where a parameter is never treated as a constant

Some procs have callers no scan can list, so their parameters are never
seeded and `I230` never fires inside them.

A `proc unknown` is the clearest case. Tcl sends it every command word
that resolves to nothing at the moment of the call — a name an
autoloader supplies, a name another sourced file introduces, a name built
by string arithmetic — and most of those appear nowhere in your source.
Whatever calls the analyser can see are only some of them, so none of
them counts as evidence:

```tcl
proc unknown {cmd args} {
    if {$cmd eq "alpha"} { return 1 } else { return 2 }
}
unknown alpha
unknown alpha
```

No `I230` is reported on the condition, even though both visible calls
pass `alpha`.

The same reasoning applies to a proc reached in a way the scan cannot
enumerate at all — through a command name it cannot read (`set cmd [gets
stdin]; $cmd dev`), or through a `namespace import`.

Two shapes that do keep their evidence, and so can still produce a fold:

- A body pinned to a namespace by `apply {params body ns}`. Its bare
  command words resolve in that namespace, so `apply {{x} { helper $x }
  ::foo} b` counts as a call to `::foo::helper`, not to a global
  `::helper`.
- A body run by `uplevel 0`. That is the *current* frame, not the global
  one — only `uplevel #0` is global — so its calls count against the
  enclosing namespace's procs.

## Where a computed variable name stops the fold

Tcl can compute a variable's *name* at run time, and the analyser cannot know
which name it lands on:

```tcl
proc parse {args} {
    foreach a $args {
        set switch [string trimleft $a -]
        set $switch {}          ;# defines whatever $switch spells
    }
    if {[info exists mixed]} { return has-mixed }   ;# no I230
    return no-mixed
}
```

`set $switch {}` may well have created `mixed`, so the check is not provably
false and no `I230` is reported. The mirror case is `unset $n`, which may
remove a variable the analyser thought certain to exist — including a
parameter — so it stops the "always true" fold in the same way. The optimiser
follows suit and leaves both branches in place, because folding away a branch
that really runs would change the program.

Spell the name out to get the fold (and the diagnostic) back.

## Where a write the analyser cannot place stops the fold

A condition is folded only over a value no write the analyser cannot place can
have replaced. Three kinds of write leave the variable undecided, and no
`I230` is reported:

```tcl
set go 1
switch -glob -- [gets stdin] { q* { set go 0 } }   ;# an arm may run
if {$go} { puts a } else { puts b }                ;# no I230

set go 1
trace add variable x write { set ::go 0 ;# }       ;# a callback writes it
set x 1
if {$go} { puts a } else { puts b }                ;# no I230 (tclsh prints b)

set g 5
foo                                                ;# a command the file never defines
if {$g} { puts a } else { puts b }                 ;# no I230, as for $::g
```

- An arm of a `switch` the flow graph keeps as one statement (`-glob`,
  `-regexp`, `-nocase`, a fall-through arm, `case`) defines every name it
  writes or binds, or that a command it runs writes into the same frame (a
  procedure that sets the caller's variable through `upvar`, `namespace eval`,
  `dict with`); the name holds its earlier value only on the paths where no
  arm runs.
- A callback script stored anywhere in the file (`after`, `fileevent`,
  `bind`, a variable trace's callback, a procedure named as one, a lambda one
  applies, a command prefix built with `list`, a script spelled as several words such as `after
  100 set done 1`) runs outside the registering code, so a name it writes is
  never a constant, in the top-level script or in any procedure. A callback
  the analyser cannot read — `after 100 $script`, a command the file does not
  define, an `interp alias`, a `{*}` expansion — may write any variable, so no
  name is decided anywhere in the file.
- A call to a command the file does not define may write, unset or read a
  name of the frame it is called from: a plain name in the top-level script is
  the global `::name`, and a procedure's local is in its reach through `upvar
  1` — a procedure an autoloader or the unknown handler brings in can do so on
  every release. So the name is undecided after such a call, as a `$::g` is. So
  it is after a call whose command is computed (`$cmd`), and a call inside the
  body of a `catch`, whether the `catch` is a command of its own or one a
  condition runs (`if {[catch {foo}]} …`), and a call inside any body a `[…]`
  substitution runs, whatever frame the body runs in: a lambda's (`[apply {{}
  {foo}}]`), a `namespace eval` or `uplevel` body, the text a `[subst
  {[foo]}]` substitutes and an expression word inside a body (`[catch {if
  {[foo]} …}]`). A name the body of a `catch` writes on some path is undecided
  afterwards too, since the body stops at its first error. A `source` runs its
  file in the frame of the call, so no name is safe across one either; a
  procedure the file defines is read for what it writes. A command a
  [stub](../kcs-howto-annotate-commands-with-stubs.md) declares is one the file
  does not define until the stub states what it does to the caller's frame: a
  plain stub (every argument a value, name, pattern or channel, no flag but
  `-pure` or `-unsafe`) with `-frame own` or `-frame none` changes no variable
  of the frame it is called from, so a name keeps its value across the call;
  one with `-frame caller` may set any variable of that frame, as `argparse`
  does, so no name in the procedure is decided.

## A test inside another test's arm

A comparison that holds proves something about its variable for the code it
guards. Inside the arm of `if {$x eq "a"}`, `x` is `a`, so a test there that
`a` never passes is constant:

```tcl
proc route {x} {
    if {$x eq "a"} {
        if {$x eq "b"} {      ;# I230: always false
            puts never
        }
    }
}
```

The same holds on the false edge of `ne`, inside `if {$x in {a b c}}`, where a
test is decided when every member answers it alike, and in each arm of an
exact `switch`. It holds only where every path crosses the test: past the `if`,
where its arms meet, `x` is undecided again, and so it is from a command that
runs a script the analyser cannot read, such as `eval $script`, which may set
`x` itself.

A numeric `==` proves a number, never a string: `1.0 == 1` is true, so inside
`if {$x == 1}` the string `x` holds may still be `1.0`, ` 1` or `01`, and a test
of its spelling there is not decided. A `::`-qualified name, a `global`,
`variable` or `upvar` alias, a traced variable, and every variable of a
procedure that computes a variable name are never narrowed: a call, a trace or
the computed name may change them between the test and the code it guards.

A plain name in top-level code is the global of that name, and it is narrowed
as a procedure's local is, on the same terms as the constant the analyser
propagates for it: a call to a command the file does not define gives it a
fresh, unknown value, and a name one of the file's procedures declares
`global` is never narrowed. A write through the qualified spelling in the same
code is not read as a write to the plain name, though, so a test of `z` made
before `set ::z c` still holds for `z` after it, as the constant a `set z`
before it gave still stands (#2370):

```tcl
set z [gets stdin]
if {$z eq "a"} {
    set ::z c
    if {$z eq "c"} { puts changed }   ;# I230: always false, though tclsh prints changed
}
```

## A test after a loop

Where the analyser knows exactly what a loop starts from — every variable it
reads holds a known value there, and every one it writes is a plain local, or
a plain name in top-level code, that nothing else can change — it runs the
loop to its end, and what the loop leaves holds after it:

```tcl
proc count {} {
    set n 0
    foreach x {a b c} { incr n }
    if {$n == 3} { return three }   ;# I230: always true
    return other
}
```

The same holds after `for` and `while`, after a `break` or a `continue`
(`for {set i 0} {$i < 10} {incr i} {if {$i == 3} break}` leaves `i` at 3), and
after a `catch` whose body raises part-way through a loop (`catch {for {set i
0} {$i < 5} {incr i} {if {$i == 2} {error x}}}` leaves `i` at 2). Inside the
loop nothing is decided from the run, since each pass sees a different value.

A loop is not run, and the test after it is decided only as it is without
the run, when it would run more than 4096 passes, reads a value the analyser
does not know (`for {set i 0} {$i < $n} {incr i} {}` with `n` a parameter),
runs a command the analyser does not evaluate (`puts`, a procedure), or
writes a `global`, traced or array variable. A command substitution in the
loop runs over the loop's values when the command registry gives its command
an evaluation, and only where it has no effect: `incr n [string length $s]`,
`incr x [expr {$b / $a}]` and `$i < [llength $l]` are run, while a
substitution whose script sets a variable (`incr n [incr k]`) or does not
complete normally stops the run, and so does one whose command has no
evaluation (`[string toupper $x]`).

A loop's own condition is never reported as always true: `while 1 { … }`
loops on purpose. That is the condition that leaves the loop when it is
false; an `if` inside the loop's body, or right after the loop, is reported
like any other. A loop's condition that is never true is reported naming the
loop — `Loop condition '$n' is never true; the loop leaves at this test` —
beside [W240](kcs-diagnostic-w240-loop-constant-false.md) where the body never
runs.

## Fix

To remember a value across calls, give it real cross-call storage instead of a
local:

```tcl
proc authorize {} {
    global handle              ;# or:  variable handle
    if {[info exists handle]} {
        return $handle
    }
    set handle [ILX::init Access-Plugin Access-Extension]
    return $handle
}
```

With `global` (or a namespace `variable`, or iRules `table` / `session` /
`static::`), the variable can persist, the existence check is no longer
constant, and `I230` disappears. If a branch really is dead, delete it.

## How to suppress

Add `# noqa: I230` on the line **above** the condition.

## Related

- [KCS codes index](README.md)
- [W210 — variable read before set](kcs-diagnostic-w210-variable-read-before-set.md)
- [O107 — unreachable dead code](kcs-optimisation-o107-unreachable-dead-code.md)
- [W240 — loop condition constant false](kcs-diagnostic-w240-loop-constant-false.md)
