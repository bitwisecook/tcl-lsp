namespace eval gone {}
namespace eval N {}
if {[catch {namespace eval N {namespace path ::gone}} message]} {
    puts [list unsupported $message]
    exit
}
namespace delete ::gone
set ::calls 0
proc fallback args {incr ::calls; return FALLBACK}
namespace eval N {
    namespace unknown ::fallback
    proc local {} {return LOCAL}
    proc localCaller {} {local}
    proc rootCaller {} {set x ROOT; return $x}
    proc missingCaller {} {missing}
}
puts [list LOCAL [N::localCaller] $::calls]
puts [list ROOT [N::rootCaller] $::calls]
puts [list MISS [N::missingCaller] $::calls]
