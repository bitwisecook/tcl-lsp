# A proc's wrong-args message names the command word as invoked (C's
# `ProcWrongNumArgs` quotes `objv[0]`), not the proc's namespace tail.
proc show {script} {
    set code [catch {uplevel 1 $script} result]
    puts "$code <$result>"
}
namespace eval a {}
proc ::a::b {x} {}
show {::a::b}
show {a::b}
show {::a::b 1 2}
show {namespace eval a {b}}
show {{::a::b}}
show {set c ::a::b; $c}
show {::a:::b}
namespace eval a {namespace export b}
namespace import a::b
show {b}
proc ::p {x {y 1}} {}
show {::p}
show {::::p}
show {namespace eval a {::p}}
proc {s p} {x} {}
show {{s p}}
namespace eval list {
    proc dispatch {cmd args} {
        uplevel 1 [linsert $args 0 [namespace current]::L$cmd]
    }
    proc Lreverse {sequence} {}
}
show {list::dispatch reverse}
