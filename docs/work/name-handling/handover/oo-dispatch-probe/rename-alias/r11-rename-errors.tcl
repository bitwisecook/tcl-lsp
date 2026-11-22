proc p {} {}
proc q {} {}
puts [list [catch {rename nosuch x} m1] $m1 [catch {rename p q} m2] $m2 [catch {rename p {}} m3]]
