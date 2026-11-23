# errorCode for reading a missing variable: compiled (in a procedure) versus top level, set versus $-substitution.
proc p {} { catch {set nosuch} m; puts "proc set: $::errorCode"; catch {puts $nosuch} m; puts "proc subst: $::errorCode" }
p
catch {set ::nosuch2} m; puts "top set qualified: $::errorCode"
catch {set nosuch3} m; puts "top set: $::errorCode"
catch {puts $nosuch4} m; puts "top subst: $::errorCode"
catch {array get nosuch5} m; puts "array get: $::errorCode"
