proc original {x} {expr {[expr {$x * 2}]}}
proc rewritten {x} {expr {$x * 2}}
foreach value {3 2.5 notANumber 0x10} {
    set firstCode [catch {original $value} first]
    set secondCode [catch {rewritten $value} second]
    puts [list normal $value $firstCode $first $secondCode $second]
}
rename expr original_expr
set calls 0
proc expr args {
    global calls
    incr calls
    uplevel 1 [linsert $args 0 original_expr]
}
set originalCode [catch {original 3} originalResult]
puts [list custom-original $originalCode $originalResult $calls]
set calls 0
set rewrittenCode [catch {rewritten 3} rewrittenResult]
puts [list custom-rewritten $rewrittenCode $rewrittenResult $calls]
