# Variables set inside a caught body and by catch's own result/options vars (W210 cases).
proc p1 {} { catch { set v 1 } ; return $v }
proc p2 {} { catch { error x } res opts ; return [list $res [dict get $opts -code]] }
proc p3 {} { if {[catch { set w [expr {1/0}] } msg]} { return "err:$msg" } ; return $w }
proc p4 {} { try { set t 5 } on error {m} { set t err } ; return $t }
proc p5 {} { try { error boom } trap NONE {m o} { return trapped } on error {em eo} { return "$em [dict get $eo -code]" } }
puts [p1]; puts [p2]; puts [p3]; puts [p4]; puts [p5]
