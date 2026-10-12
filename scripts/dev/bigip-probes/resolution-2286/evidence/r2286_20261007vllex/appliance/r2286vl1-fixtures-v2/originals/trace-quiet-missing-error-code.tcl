proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set errorCode SENTINEL; set h lappend; $h v; list [set errorCode] [set v]} r]
set summary [list $c [hex $r]]
puts $summary
set summary
