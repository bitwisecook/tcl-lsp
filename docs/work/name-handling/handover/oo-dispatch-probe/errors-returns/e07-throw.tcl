# throw sets errorCode; an empty type is an error.
puts [list [catch {throw {MY ERR 42} message} m o] $m [dict get $o -errorcode] $::errorCode]
puts [list [catch {throw {} message} m] $m]
puts [list [catch {throw X} m] $m]
