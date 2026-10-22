namespace eval n {}
proc n:: {} {return EMPTY}
interp alias {} a {} n::
puts [a]
