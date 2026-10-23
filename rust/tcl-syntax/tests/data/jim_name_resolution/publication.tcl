proc hx {v} {binary scan $v H* result; return $result}
set ctx [binary format H* {@CONTEXT@}]
set name [binary format H* {@NAME@}]
set before [info -nons procs]
namespace eval $ctx [list proc $name {} {return [namespace canonical]}]
set published {}
foreach candidate [info -nons procs] {if {[lsearch -exact $before $candidate] < 0} {lappend published $candidate}}
if {[llength $published] != 1} {error "ambiguous publication: $published"}
set key [lindex $published 0]
puts "[hx $key]|[hx [namespace eval $ctx [list $name]]]"
