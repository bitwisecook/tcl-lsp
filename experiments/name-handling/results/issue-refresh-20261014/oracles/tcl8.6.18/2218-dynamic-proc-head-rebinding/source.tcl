proc double_expr {x} {return [expr {[expr {$x * 2}]}]}
proc g {cmd} {$cmd}
proc change {} {rename ::expr ::realexpr; proc ::expr {args} {return [lindex $args 0]}}
g change
puts [double_expr 2]
