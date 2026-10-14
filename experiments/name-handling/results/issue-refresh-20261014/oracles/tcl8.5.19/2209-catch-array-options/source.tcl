catch {set x ok} result options(key)
puts [list $result [info exists options(key)] [dict get $options(key) -code]]
