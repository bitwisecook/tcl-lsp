proc a {} { set m amark; b }
proc b {} { tailcall c }
proc c {} { list [info level] [uplevel 1 {info exists m}] }
puts [a]
