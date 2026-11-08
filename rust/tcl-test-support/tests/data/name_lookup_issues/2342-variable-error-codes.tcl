catch {set nosuch} m o
puts [dict get $o -errorcode]
catch {unset nosuch2} m o
puts [dict get $o -errorcode]
array set arr {a 1}
catch {set arr} m o
puts [dict get $o -errorcode]
