proc p {a b} {upvar 1 $a v; set v 1; upvar 1 $b v; return $v}
set x 0; set y 7
puts [catch {p x y} r]:$r:$x:$y
