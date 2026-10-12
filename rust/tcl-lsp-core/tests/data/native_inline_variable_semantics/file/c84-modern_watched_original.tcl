proc observe {args} {puts OBSERVED}
if {[catch {trace add variable x read observe} msg]} {puts UNAVAILABLE; puts $msg} else {set x 1; puts $x}
