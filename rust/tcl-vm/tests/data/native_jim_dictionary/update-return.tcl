set code [catch {proc p {} {set d {k 1}; dict update d k x {return Z}; return END}; p} result options]
puts $code
puts -nonewline $result
