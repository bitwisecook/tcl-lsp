# A for loop directly inside a catch body: its loop variable is set by the init clause before the test reads it.
proc a {} { catch {for {set i 0} {$i < 3} {incr i} {}} r ; return "a:$r:$i" }
proc b {} { if {[catch { for {set k 0} {$k < 2} {incr k} {} } m]} { return $m } ; return "b:$k" }
proc c {} { catch {for {set j 0} {$j < 3} {break} { set z $j }} ; return "c:$z" }
puts [a]; puts [b]; puts [c]
