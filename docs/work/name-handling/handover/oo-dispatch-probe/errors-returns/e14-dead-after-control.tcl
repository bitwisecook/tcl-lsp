# Code after error, return, break, throw (analyser: unreachable code).
proc a {} { return 1; puts unreachable-a }
proc b {} { error x; puts unreachable-b }
proc c {} { foreach i {1} { break; puts unreachable-c } ; return c }
proc d {} { throw {X} y; puts unreachable-d }
proc e {} { if {1} { return e } else { return e2 } ; puts unreachable-e }
proc f {} { catch {error x} ; return reached-f }
puts [a]; puts [catch b]; puts [c]; puts [catch d]; puts [e]; puts [f]
