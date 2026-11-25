set code [catch {namespace eval ::n {}; set auto_index(::n::leaf) {proc ::n::leaf {} {return LOADED}}; ::n::leaf} result]
binary scan $result H* hex
puts [list $code $hex]
