proc add {a b} {
    return [expr {$a + $b}]
}

proc passthrough {x} {
    return $x
}

set timeout 30
set half [expr {$timeout / 2}]
set threshold [expr {$timeout + 10}]
set candidate [expr {$request_count + 1 + 2}]
set route [passthrough 42]
#   ^--- cursor

set banner {Hello}
append banner { }
append banner World

set stale 1
set stale 2
puts $stale

set rolling 1
set rolling [expr {$rolling + 1}]
set rolling 5
puts $rolling

if {0} {
    puts never
    set never_seen 1
}
puts always

if {[add 1 2] == 3} {
    puts matched
}
