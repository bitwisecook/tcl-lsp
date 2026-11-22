proc other {} {return aliased}
interp alias {} x {} other
proc p {} {return p}
puts [list [p] [x]]
