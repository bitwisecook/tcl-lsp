proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set v {OLD KEEP}; proc watch {n k op} {uplevel 1 {unset v; set v {NEW KEEP}}}; trace variable v r watch; set h lset; set answer [$h v 1 EXTRA]; list $answer [set v]} r]
set summary [list $c [hex $r]]
set summary
