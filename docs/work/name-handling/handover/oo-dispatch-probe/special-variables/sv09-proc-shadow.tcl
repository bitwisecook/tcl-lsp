# A special name inside a procedure is an ordinary local unless linked.
proc a {} { return [catch {set tcl_version} m]:$m }
proc b {} { global tcl_version; return $tcl_version }
proc c {} { return $::tcl_version }
proc d {} { upvar #0 tcl_platform p; return $p(platform) }
puts [a]
puts [b]
puts [c]
puts [d]
