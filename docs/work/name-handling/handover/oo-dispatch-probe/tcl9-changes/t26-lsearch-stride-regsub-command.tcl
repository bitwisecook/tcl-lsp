puts [catch {lsearch -stride 2 -index 1 {a 1 b 2} 2} r]:$r
puts [catch {regsub -all -command {\d} a1b2 {apply {x {expr {$x*2}}}}} r]:$r
