set l {a {b c} d}
lset l 1 0 X; lset l end Z
set e {}; lset e end+1 new
puts [list $l $e [catch {lset l 9 q} m] $m]
