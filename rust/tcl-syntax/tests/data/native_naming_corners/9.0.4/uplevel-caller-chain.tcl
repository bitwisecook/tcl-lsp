set code [catch {proc outer {} {set local OUTER; inner}; proc inner {} {set local INNER; list [uplevel 1 {set local}] [uplevel #0 {namespace current}]}; outer} result]
binary scan $result H* hex
puts [list $code $hex]
