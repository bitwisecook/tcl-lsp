set code [catch {namespace eval ::n {}; proc ::n:::leaf {} {return FOUND}; list [::n:::leaf] [namespace which -command ::n:::leaf]} result]
binary scan $result H* hex
puts [list $code $hex]
