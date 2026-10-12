puts [list provider [info patchlevel]]
set ::seen {}
proc cmp {a b} {lappend ::seen ROOT; return 0}
namespace eval ::A {
    proc cmp {a b} {lappend ::seen A [uplevel 1 {info level}] [uplevel 1 {namespace current}]; return 0}
    proc sort {} {lsort -command cmp {b a}}
}
set code [catch {::A::sort} result]
puts [list lsort $code $result $::seen]
set ::seen {}
proc delayed {} {lappend ::seen ROOT [info level] [namespace current]}
namespace eval ::A {
    proc delayed {} {lappend ::seen A}
    proc queue {} {after idle delayed}
}
set code [catch {::A::queue; update} result]
puts [list after $code $result $::seen]
set ::seen {}
proc watch {n1 n2 op} {lappend ::seen ROOT}
namespace eval ::A {proc watch {n1 n2 op} {lappend ::seen A}}
namespace eval ::B {
    proc watch {n1 n2 op} {lappend ::seen B [uplevel 1 {info level}] [uplevel 1 {namespace current}]}
    proc trigger {} {set ::observed VALUE}
}
set code [catch {namespace eval ::A {trace add variable ::observed write watch}; ::B::trigger} result]
puts [list trace $code $result $::seen]
set ::seen {}
proc observedCommand {} {return VALUE}
proc watchCommand {old new op} {lappend ::seen ROOT}
namespace eval ::A {proc watchCommand {old new op} {lappend ::seen A}}
namespace eval ::B {
    proc watchCommand {old new op} {lappend ::seen B [uplevel 1 {info level}] [uplevel 1 {namespace current}]}
    proc triggerCommand {} {rename ::observedCommand ::renamedCommand}
}
set code [catch {namespace eval ::A {trace add command ::observedCommand rename watchCommand}; ::B::triggerCommand} result]
puts [list command-trace $code $result $::seen]
set ::seen {}
proc observedExecution {value} {return $value}
proc watchExecution {cmd op} {lappend ::seen ROOT}
namespace eval ::A {proc watchExecution {cmd op} {lappend ::seen A}}
namespace eval ::B {
    proc watchExecution {cmd op} {lappend ::seen B [uplevel 1 {info level}] [uplevel 1 {namespace current}]}
    proc triggerExecution {} {::observedExecution VALUE}
}
set code [catch {namespace eval ::A {trace add execution ::observedExecution enter watchExecution}; ::B::triggerExecution} result]
puts [list execution-trace $code $result $::seen]
