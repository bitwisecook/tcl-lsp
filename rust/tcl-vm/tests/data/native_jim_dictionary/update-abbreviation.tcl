set code [catch {set d {k 1}; dict up d k x {}} result options]
puts $code
puts -nonewline $result
