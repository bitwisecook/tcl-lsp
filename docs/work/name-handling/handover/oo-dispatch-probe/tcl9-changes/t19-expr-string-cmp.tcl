# TIP 461: lt gt le ge
puts [catch {expr {"a" lt "b"}} r]:$r
puts [catch {expr {"10" le "9"}} r]:$r
