namespace eval Target {variable x ROOT_TARGET}
namespace eval N {
    namespace eval Target {variable x N_TARGET}
    proc p {} {
        uplevel #0 {puts [list root $Target::x]}
        uplevel 0 {puts [list current $Target::x]}
        uplevel 1 {puts [list caller $Target::x]}
        uplevel 1 {puts [list explicit-root $::Target::x]}
    }
}
namespace eval Caller {
    namespace eval Target {variable x CALLER_TARGET}
    proc c {} {::N::p}
}
::Caller::c
