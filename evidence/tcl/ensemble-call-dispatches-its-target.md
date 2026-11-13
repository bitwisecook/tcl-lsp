# Tcl source-file execution evidence

## Question

> A call of a literal `namespace ensemble create` command is a call of the command its subcommand selects, so a static analysis can walk it like an interpreter alias and keep knowing the command world after it, if it knows exactly how the target is chosen and looked up.

> How does a literal ensemble look up a relative `-map` target, an unmapped `-subcommands` entry, a duplicate `-map` key and a `-map` key that overrides a `-subcommands` entry; are the prefix's extra words and the remaining call words passed on; does a miss run anything without `-unknown`, and does `-unknown` run for a known subcommand; is a command the target creates visible after the call; does `configure` change the dispatch; does a missing target reach `unknown`; and which words must follow a renamed target?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> Each line prints the results of calls that show which implementation the ensemble selected, or the error it raised. Tcl 8.4 has no `namespace ensemble` (evidence/tcl/ensemble-create-publishes-one-command.md) and was not run. `-parameters`, export-based ensembles, Jim and BIG-IP were not tested here.

Other assumptions were not separately recorded.

## Script

```tcl-visible
proc ::Foo {args} {return "::Foo $args"}
namespace eval ::e {
    proc Foo {args} {return "::e::Foo $args"}
    namespace ensemble create -map {foo Foo two {Foo a b} gl Gl dup ::Foo dup Foo}
}
namespace eval ::other {proc Foo {args} {return "::other::Foo $args"}}
proc ::Gl {} {return ::Gl}
puts "map [namespace eval ::other {e foo x}] | [e two c d] | [e dup] | [catch {e gl} m] $m"
proc ::zz {} {return ::zz}
namespace eval ::s {namespace ensemble create -subcommands {zz}}
puts "subcommands [catch {s zz} m] $m"
namespace eval ::m {
    proc x {} {return ::m::x}
    proc y {} {return ::m::y}
    namespace ensemble create -subcommands {x} -map {x ::m::y go ::m::x go ::m::y}
}
puts "override [m x] [catch {m go} m] $m"
namespace eval ::u {
    proc go {} {return go}
    namespace ensemble create -subcommands {go} -unknown ::handler
}
proc ::handler {args} {puts "handler $args"; return {}}
namespace eval ::v {namespace ensemble create -map {go ::u::go}}
puts "miss [u go] [catch {v zz} m] $m | [catch {v} m] $m"
namespace eval ::w {
    proc mk {} {proc ::made {} {return made}}
    namespace ensemble create -map {mk ::w::mk}
}
w mk
puts "world [made]"
namespace eval ::c {namespace ensemble create -map {go ::c::A}}
proc ::c::A {} {return A}
proc ::c::B {} {return B}
set before [c go]
namespace ensemble configure ::c -map {go ::c::B}
puts "configure $before [c go]"
rename ::unknown ::_unknown
proc ::unknown {args} {return "unknown $args"}
namespace eval ::n {namespace ensemble create -map {go ::n::Missing}}
puts "missing [n go 1]"
rename ::unknown {}
rename ::_unknown ::unknown
namespace eval ::app::widget {
    proc alpha {} {return alpha}
    namespace ensemble create -command ::app::widget -subcommands {alpha}
}
rename ::app::widget::alpha ::app::widget::beta
puts "renamed-subcommand [catch {::app::widget alpha} m] $m | [catch {::app::widget beta} m] $m"
proc ::app::Show {} {return showing}
namespace ensemble create -command ::app::map -map {show ::app::Show}
rename ::app::Show ::app::Display
namespace ensemble configure ::app::map -map {show ::app::Display}
puts "renamed-map [::app::map show]"
```

## Results

### tcl8.5, run 1

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
map ::e::Foo x | ::e::Foo a b c d | ::e::Foo  | 1 invalid command name "::e::Gl"
subcommands 1 invalid command name "::s::zz"
override ::m::y 1 unknown or ambiguous subcommand "go": must be x
miss go 1 unknown or ambiguous subcommand "zz": must be go | 1 wrong # args: should be "v subcommand ?argument ...?"
world made
configure A B
missing unknown ::n::Missing 1
renamed-subcommand 1 invalid command name "::app::widget::alpha" | 1 unknown or ambiguous subcommand "beta": must be alpha
renamed-map showing
```

- Standard error: empty.

### tcl8.6, run 2

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
map ::e::Foo x | ::e::Foo a b c d | ::e::Foo  | 1 invalid command name "::e::Gl"
subcommands 0 ::zz
override ::m::y 1 unknown or ambiguous subcommand "go": must be x
miss go 1 unknown or ambiguous subcommand "zz": must be go | 1 wrong # args: should be "v subcommand ?arg ...?"
world made
configure A B
missing unknown ::n::Missing 1
renamed-subcommand 1 invalid command name "alpha" | 1 unknown or ambiguous subcommand "beta": must be alpha
renamed-map showing
```

- Standard error: empty.

### tcl9.0, run 3

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
map ::e::Foo x | ::e::Foo a b c d | ::e::Foo  | 1 invalid command name "::e::Gl"
subcommands 0 ::zz
override ::m::y 1 unknown or ambiguous subcommand "go": must be x
miss go 1 unknown or ambiguous subcommand "zz": must be go | 1 wrong # args: should be "v subcommand ?arg ...?"
world made
configure A B
missing unknown ::n::Missing 1
renamed-subcommand 1 invalid command name "alpha" | 1 unknown or ambiguous subcommand "beta": must be alpha
renamed-map showing
```

- Standard error: empty.

### tcl9.1, run 4

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
map ::e::Foo x | ::e::Foo a b c d | ::e::Foo  | 1 invalid command name "::e::Gl"
subcommands 0 ::zz
override ::m::y 1 unknown or ambiguous subcommand "go": must be x
miss go 1 unknown or ambiguous subcommand "zz": must be go | 1 wrong # args: should be "v subcommand ?arg ...?"
world made
configure A B
missing unknown ::n::Missing 1
renamed-subcommand 1 invalid command name "alpha" | 1 unknown or ambiguous subcommand "beta": must be alpha
renamed-map showing
```

- Standard error: empty.
