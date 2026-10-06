set ::env(ITCL_LIBRARY) [lindex $argv 0]
load [lindex $argv 1] Itcl
proc collect {ns} {set names [info commands ${ns}::*];foreach child [namespace children $ns] {lappend names {*}[collect $child]};return $names}
puts [join [lsort -unique [collect ::itcl]] \n]
