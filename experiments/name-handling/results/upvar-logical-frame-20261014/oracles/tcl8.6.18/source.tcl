proc helper {source} {upvar 1 $source local; set local FORMAL}
proc caller {} {helper destination; puts $destination}
caller
proc literal {} {upvar 1 {$scalar(open} {$alias}; set {$alias} LITERAL}
proc literalcaller {} {literal; puts [set {$scalar(open}]}
literalcaller
