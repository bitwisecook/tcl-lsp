# Compiler Explorer Demo
#
# Open this file alongside the compiler explorer panel to see:
#   IR        — structured intermediate representation
#   CFG       — control-flow graph with loop back-edges and branches
#   SSA       — phi nodes where variables merge after branches/loops
#   Optimiser — constant propagation, dead-store elimination, lassign packing

proc fibonacci {n} {
    set a 0
    set b 1
    for {set i 0} {$i < $n} {incr i} {
        set next [expr {$a + $b}]
        set a $b
        set b $next
    }
    return $b
}

proc classify {value} {
    set threshold 100
    set upper [expr {$threshold * 2}]
    if {$value > $upper} {
        set tier "premium"
    } elseif {$value > $threshold} {
        set tier "standard"
    } else {
        set tier "basic"
    }
    return $tier
}

puts [fibonacci 10]
puts [classify 150]
