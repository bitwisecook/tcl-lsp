proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set v OLD; proc watch {n k op} {uplevel 1 {unset v; set v NEW}}; trace variable v r watch; set h lappend; set answer [$h v]; list $answer [set v]} r]
set summary [list $c [hex $r]]
set summary
