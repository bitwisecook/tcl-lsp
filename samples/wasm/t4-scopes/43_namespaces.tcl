# T4: namespace eval, namespace variables, qualified procs, ensembles.
namespace eval ::geo {
    variable pi 3.14159
    proc area {r} {
        variable pi
        expr {$pi * $r * $r}
    }
    proc circumference {r} { variable pi; expr {2 * $pi * $r} }
    namespace export area circumference
    namespace ensemble create
}
puts [::geo::area 1]
puts [geo circumference 1]
puts [namespace exists ::geo]
puts [lsort [info commands ::geo::*]]
namespace eval ::geo { variable pi 3 }
puts [geo area 2]
