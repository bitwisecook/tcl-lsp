set code [catch {set d {k 1}; dict incr d k; set d} result options]
puts $code
puts -nonewline $result
