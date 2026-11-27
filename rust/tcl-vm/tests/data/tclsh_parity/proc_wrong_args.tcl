# A procedure's `wrong # args` message names it by the command word it was
# invoked as (C's `objv[0]`), not by its namespace tail.
proc show {script} {
    set code [catch {uplevel 1 $script} result]
    puts "$code <$result>"
}
namespace eval a {}
proc ::a::b {x} {}
proc top {x {y 1} args} {}
proc {a b} {x} {}
namespace eval a { proc c {x} {} }
show {::a::b}
show {a::b}
show {::a::b 1 2}
show {::a::::b}
show {top}
show {::top}
show {{a b}}
show {namespace eval a {b}}
show {namespace eval a {c}}
show {namespace eval a {::a::c}}
show {set cmd ::a::b; $cmd}
show {eval ::a::b}
show {uplevel #0 ::a::b}
show {namespace eval a {namespace export c}; namespace import ::a::c; c}
show {rename ::a::b ::a::bb; ::a::bb}
# tcllib's `struct::list` dispatch: the subcommand runs as its full name.
namespace eval ::struct::list {}
proc ::struct::list::Lreverse {sequence} {}
proc ::struct::list::list {cmd args} {
    uplevel 1 [linsert $args 0 ::struct::list::L$cmd]
}
show {::struct::list::list reverse}
