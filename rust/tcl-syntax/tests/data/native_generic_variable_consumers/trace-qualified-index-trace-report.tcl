proc hex {s} {binary scan $s H* out; return $out}
set c [catch {namespace eval N {}; set log {}; proc watch {n k op} {global log; lappend log [list $n $k $op]}; set n {::N::arr(k::part)}; trace variable $n rw watch; set h lappend; $h $n A B; list $log [set $n]} r]
set summary [list $c [hex $r]]
puts $summary
set summary
