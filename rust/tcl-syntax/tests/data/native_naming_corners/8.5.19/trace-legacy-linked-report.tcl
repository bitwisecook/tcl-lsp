set code [catch {set a(k) VALUE; set log {}; proc observe {n e op} {lappend ::log [list $n $e $op]}; proc p {} {upvar #0 a(k) alias; trace variable alias r observe; set alias}; list [p] $log} result]
binary scan $result H* hex
puts [list $code $hex]
