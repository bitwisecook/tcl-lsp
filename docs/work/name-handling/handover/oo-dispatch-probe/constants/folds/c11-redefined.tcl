proc f {} { return one }
proc p {} { set a [f]; proc ::f {} { return two }; set b [f]; return [list $a $b] }
puts [p]
