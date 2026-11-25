set code [catch {namespace eval ::n::::child {list [namespace current] [namespace parent] [namespace tail ::n::::child]}} result]
binary scan $result H* hex
puts [list $code $hex]
