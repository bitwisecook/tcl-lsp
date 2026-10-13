set x ROOT
namespace eval N {
    variable x NAMESPACE
    proc p {} {
        set x PROCEDURE
        list [uplevel #0 {set x}] [uplevel 0 {set x}]
    }
    set direct [uplevel #0 {set x}]
}
puts [list namespace-proc [N::p] direct-namespace $N::direct]
oo::class create C {
    method m {} {
        set x METHOD
        list [uplevel #0 {set x}] [uplevel 0 {set x}]
    }
}
C create o
puts [list method [o m]]
proc caller {} {
    set x CALLER
    callee
}
proc callee {} {
    set x CALLEE
    uplevel 1 {list [set x] [set y INNER]}
}
puts [list actual-caller [caller]]
