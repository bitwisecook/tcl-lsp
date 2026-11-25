set code [catch {namespace eval ::n {variable x VALUE}; proc p {} {namespace upvar ::n x alias; set alias}; p} result]
binary scan $result H* hex
puts [list $code $hex]
