set c [catch {package require tcltest} r]
binary scan $r H* h
puts [list $c $h]
