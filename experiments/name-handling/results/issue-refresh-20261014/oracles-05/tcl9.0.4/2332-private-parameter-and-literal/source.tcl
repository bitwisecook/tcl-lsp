set seen GLOBAL
proc local {seen} {return $seen}
proc literal {} {return {$::seen}}
puts [local FIRST]
set seen CHANGED
puts [local SECOND]
puts [literal]
