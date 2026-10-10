proc observe {args} {puts OBSERVED}
if {[catch {trace add variable x read observe} msg]} {puts UNAVAILABLE; puts $msg} else {puts 1}
