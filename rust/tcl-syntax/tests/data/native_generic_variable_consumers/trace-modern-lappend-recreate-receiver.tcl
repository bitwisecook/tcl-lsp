proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set v OLD; proc watch {n k op} {uplevel 1 {unset v; set v NEW}}; trace add variable v read watch; set h lappend; set answer [$h v EXTRA]; list $answer [set v]} r]
set summary [list $c [hex $r]]
puts $summary
set summary
