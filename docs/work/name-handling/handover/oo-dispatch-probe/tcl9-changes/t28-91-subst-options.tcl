# 9.1 TIP 712: subst -backslashes/-commands/-variables
set v V
puts [catch {subst -variables {$v [string length x] \t}} r]:$r
