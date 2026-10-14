proc p {first {"as" ""} {third value}} {return FIRST}
puts [list defaults_and_quoted [info args p]]
proc p "other {last default}" {return SECOND}
puts [list quoted_parent [info args p]]
proc p "first\x20second" {return VALUE}
puts [list escaped_parent [info args p]]
proc p {"as" value extra} {return 1}
puts [list quoted_marker [info args p]]
