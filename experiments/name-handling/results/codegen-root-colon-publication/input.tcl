proc :audit {} {return ORIGINAL}
puts [list original-colon [catch {:audit} r] $r]
puts [list original-plain [catch {audit} r] $r]
proc :::transport {} {return TRANSPORT}
puts [list transport-colon [catch {:transport} r] $r]
puts [list transport-plain [catch {transport} r] $r]
