set code [catch {namespace eval ::n {}; proc ::n:::edge {} {return EDGE}; list [::n:::edge] [info procs ::n::*]} result]
binary scan $result H* hex
puts [list $code $hex]
