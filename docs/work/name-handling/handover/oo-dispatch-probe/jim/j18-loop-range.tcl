puts [catch {loop i 0 3 {puts -nonewline $i}} r]:$r
puts ""
puts [catch {range 3} r]:$r
