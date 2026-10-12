puts "VERSION|[info patchlevel]"
set ::observed {}
namespace eval Registrar {
    package ifneeded R2286FutureFrame 1.0 {
        lappend ::observed [list [namespace current] [info level] [info exists localSentinel]]
        package provide R2286FutureFrame 1.0
    }
}
namespace eval Caller {
    proc run {} {
        set localSentinel local
        package require R2286FutureFrame
    }
}
set code [catch {Caller::run} result]
puts [list RESULT $code $result $::observed]
