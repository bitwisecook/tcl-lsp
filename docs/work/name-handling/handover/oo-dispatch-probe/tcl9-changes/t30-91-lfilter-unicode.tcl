# 9.1 TIP 735 lfilter, TIP 726 unicode
puts [catch {lfilter x {1 2 3 4} {expr {$x % 2}}} r]:$r
puts [catch {lfilter x {1 2 3 4} {$x % 2}} r]:$r
puts [catch {string length [unicode tonfd "é"]} r]:$r
