set code [catch {namespace eval ::n {variable x VALUE}; proc p {} {variable ::n::x; list [info locals] [set x]}; p} result]
binary scan $result H* hex
puts [list $code $hex]
