set k dynamic
set a(dynamic) 23
set {a($k)} 19
puts [list literal-dollar-index [expr {${a($k)}}]]
puts [list substituted-index [expr {$a($k)}]]
set inner k
set b(k) dynamic
puts [list nested-index [expr {$a($b($inner))}]]
set calls 0
proc index {} {global calls; incr calls; return dynamic}
set {a([index])} 31
puts [list literal-command-index [expr {${a([index])}}] $calls]
puts [list substituted-command-index [expr {$a([index])}] $calls]
set k 41
puts [list quoted-variable [expr {"pre$k"}]]
puts [list braced-no-substitution [expr {{pre$k}}]]
puts [list quoted-array [expr {"$a(dynamic)"}]]
