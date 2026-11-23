# 8.7/9.0: lpop, lremove, ledit
set l {a b c d}
puts [catch {lpop l} r]:$r:$l
set l {a b c d}
puts [catch {lremove $l 0 2} r]:$r
set l {a b c d}
puts [catch {ledit l 1 2 X Y Z} r]:$r:$l
