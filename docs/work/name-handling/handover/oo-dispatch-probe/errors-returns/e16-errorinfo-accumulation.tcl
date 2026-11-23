# errorInfo grows through procedure frames, eval and uplevel.
proc a {} { eval {uplevel 1 {error deep}} }
proc b {} { a }
catch {b} m
foreach line [split $::errorInfo \n] { puts "| $line" }
