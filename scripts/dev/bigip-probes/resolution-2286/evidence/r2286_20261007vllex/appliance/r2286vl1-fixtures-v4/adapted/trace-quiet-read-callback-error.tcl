proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set errorCode SENTINEL; set v OLD; proc watch {n k op} {error BOOM}; trace variable v r watch; set h lappend; set answer [$h v]; trace vdelete v r watch; list $answer [set errorCode] [set v]} r]
set summary [list $c [hex $r]]
set summary
