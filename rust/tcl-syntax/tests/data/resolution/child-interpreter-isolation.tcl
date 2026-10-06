namespace eval ::N {proc probe {} {return PARENT}}
package provide OracleIsolated 2.0
if {[catch {interp create} child]} {
    set child [interp]
    set handle_api 1
} else {
    set handle_api 0
}
$child eval {namespace eval ::N {proc probe {} {return CHILD}}}
puts [list [::N::probe] [$child eval {::N::probe}] [$child eval {catch {package require OracleIsolated}}]]
if {$handle_api} {$child delete} else {interp delete $child}
