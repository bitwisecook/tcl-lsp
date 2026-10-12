proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set errorCode SENTINEL; set v OLD; proc watch {n k op} {error BOOM}; trace add variable v read watch; set h lappend; set answer [$h v]; trace remove variable v read watch; list $answer [set errorCode] [set v]} r]
set summary [list $c [hex $r]]
puts $summary
set summary
