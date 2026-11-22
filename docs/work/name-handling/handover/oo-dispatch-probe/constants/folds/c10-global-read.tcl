set ::g 7
proc readg {} { return $::g }
proc p {} { set a [readg]; set ::g 8; set b [readg]; return [list $a $b] }
puts [p]
