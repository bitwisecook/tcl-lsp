# TIP 551: digit separators in numbers (9.0)
puts [catch {expr {1_000 + 1}} r]:$r
puts [catch {string is integer -strict 1_000} r]:$r
puts [catch {incr x 1_0} r]:$r
