proc observe {args} {puts OBSERVED}
if {[catch {trace variable x r observe} msg]} {puts UNAVAILABLE; puts $msg} else {puts 1}
