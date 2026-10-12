proc observe {args} {puts OBSERVED}
if {[catch {trace variable x r observe} msg]} {puts UNAVAILABLE; puts $msg} else {set x 1; puts $x}
