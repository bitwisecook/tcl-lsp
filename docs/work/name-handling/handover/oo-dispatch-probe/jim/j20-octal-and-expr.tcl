puts [expr {010 + 0}]
puts [catch {expr {0o10}} r]:$r
puts [expr {7 / 2}]:[expr {-7 / 2}]
puts [expr {2 ** -1}]
