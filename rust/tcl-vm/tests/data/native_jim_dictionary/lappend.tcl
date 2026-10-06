set code [catch {set d {k A}; dict lappend d k B; set d} result options]
puts $code
puts -nonewline $result
