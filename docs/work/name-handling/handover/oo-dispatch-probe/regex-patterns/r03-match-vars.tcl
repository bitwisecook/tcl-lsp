# match and sub-match variables, unmatched groups, -indices
set r [regexp {(a)(b)?(c)} "ac" m s1 s2 s3]
puts [list $r $m $s1 $s2 $s3]
regexp -indices {(a)(b)?(c)} "ac" m s1 s2 s3
puts [list $m $s1 $s2 $s3]
set r [regexp {zzz} "abc" m2]
puts [list $r [info exists m2]]
set m3 keep
regexp {(q)} "abc" m3
puts [list m3 $m3]
regexp {(a)} "a" whole
puts [list whole $whole]
proc p {} { if {[regexp {(\d+)-(\d+)} "10-20" -> lo hi]} { return [expr {$lo + $hi}] }; return none }
puts [p]
puts [catch {regexp -all -inline {(a)} "aa" v} m]$m
puts [regexp -all {(a)(b)} "abab" w x y]:$w:$x:$y
