set code [catch {namespace eval ::source {proc leaf {} {return SRC}; namespace export leaf}; namespace eval ::target {namespace import ::source::leaf; set before [leaf]; namespace forget ::source::leaf; list $before [info commands leaf]}} result]
binary scan $result H* hex
puts [list $code $hex]
