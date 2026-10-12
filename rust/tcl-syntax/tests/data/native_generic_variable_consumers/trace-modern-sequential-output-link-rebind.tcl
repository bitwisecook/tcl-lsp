proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set replacement BEFORE; set first INIT; set second ORIGINAL; proc watch {n k op} {uplevel 1 {unset second; upvar #0 replacement second}}; trace add variable first write watch; set n first; set m second; set h scan; set count [$h {7 9} {%d %d} $n $m]; list $count $first $second $replacement} r]
set summary [list $c [hex $r]]
puts $summary
set summary
