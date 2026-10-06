set code [catch {set r {}; dict for {k v} {a 1 b 2} {lappend r $k $v}; set r} result options]
puts $code
puts -nonewline $result
