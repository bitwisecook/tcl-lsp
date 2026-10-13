proc audit {} {return FIRST}
puts [list before [audit]]
proc audit {} {return SECOND}
puts [list after [audit]]
