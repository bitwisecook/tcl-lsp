set code [catch {namespace eval ::source {proc leaf {} {return SRC}; namespace export leaf}; namespace eval ::target {proc leaf {} {return OLD}; namespace import -force ::source::leaf}; rename ::source::leaf ::source::moved; list [::target::leaf] [namespace origin ::target::leaf]} result]
binary scan $result H* hex
puts [list $code $hex]
