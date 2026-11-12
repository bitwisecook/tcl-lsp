# Tcl source-file execution evidence

## Question

> `namespace ensemble create` changes the command table, and a static analysis that cannot say how must stop trusting every later command reference. Knowing exactly which command it installs, and that nothing else changes, lets the LSP keep later references and publish only that one slot.

> With every option literal, which commands does `namespace ensemble create` add or remove, with no `-command`, with a relative `-command` naming an existing procedure, and with an absolute `-command`; does a rejected `-map` value create anything; does `namespace ensemble configure` add or remove any command; and does a later `proc` of the ensemble's name replace it?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> Each line lists the commands added and removed in `::`, `::e` and `::q` between two snapshots of `info commands`, followed by results of calls that show which implementation a name selects. Option abbreviations, computed operands, `-parameters`, `-subcommands`, `-unknown`, Jim and BIG-IP were not tested here.

Other assumptions were not separately recorded.

## Script

```tcl-visible
proc commands {} {lsort [concat [info commands ::*] [info commands ::e::*] [info commands ::q::*]]}
proc changed {before} {
    set after [commands]
    set added {}
    foreach name $after {if {[lsearch -exact $before $name] < 0} {lappend added $name}}
    set removed {}
    foreach name $before {if {[lsearch -exact $after $name] < 0} {lappend removed $name}}
    list $added $removed
}
namespace eval ::e {}
namespace eval ::q {}
proc ::e::Foo {args} {return "foo: $args"}
proc ::q::old {} {return old}
proc ::q::keep {} {return keep}
set before [commands]
namespace eval ::e {namespace ensemble create -map {foo ::e::Foo}}
puts "default [changed $before] [e foo bar]"
set before [commands]
namespace eval ::q {namespace ensemble create -command old -map {k ::q::keep}}
puts "replace [changed $before] [::q::old k] [namespace eval ::q {keep}]"
set before [commands]
set r [namespace eval ::q {namespace ensemble create -command ::top}]
puts "absolute $r [changed $before]"
set before [commands]
set code [catch {namespace eval ::q {namespace ensemble create -command bad -map {a {}}}} msg]
puts "invalid $code [changed $before]"
set before [commands]
namespace ensemble configure ::e -map {foo ::q::keep} -prefixes 0
puts "configure [changed $before] [e foo]"
proc ::e {} {return replaced}
puts "proc [e]"
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 1
- Standard output: empty.
- Standard error:

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -map {foo ::e::Foo}"
    (in namespace eval "::e" script line 1)
    invoked from within
"namespace eval ::e {namespace ensemble create -map {foo ::e::Foo}}"
    (file "ensemble-create-publishes-one-command.tcl" line 16)
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
default ::e {} foo: bar
replace {} {} keep keep
absolute ::top ::top {}
invalid 1 {} {}
configure {} {} keep
proc replaced
```

- Standard error: empty.

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
default ::e {} foo: bar
replace {} {} keep keep
absolute ::top ::top {}
invalid 1 {} {}
configure {} {} keep
proc replaced
```

- Standard error: empty.

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
default ::e {} foo: bar
replace {} {} keep keep
absolute ::top ::top {}
invalid 1 {} {}
configure {} {} keep
proc replaced
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
default ::e {} foo: bar
replace {} {} keep keep
absolute ::top ::top {}
invalid 1 {} {}
configure {} {} keep
proc replaced
```

- Standard error: empty.
