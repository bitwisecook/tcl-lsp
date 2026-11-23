# Code after tailcall is unreachable; tailcall inside catch still replaces the frame.
proc after_tc {} { tailcall list tc; puts unreachable; return never }
puts [after_tc]
proc in_catch {} { set rc [catch {tailcall list inside} r]; return "rc=$rc r=$r" }
puts [in_catch]
proc arity {} { tailcall }
puts [list [catch arity m] $m]
