set code [catch {proc leaf {} {return ORIGINAL}; namespace eval ::n {}; rename leaf ::n:::moved; list [::n:::moved] [info commands ::leaf]} result]
binary scan $result H* hex
puts [list $code $hex]
