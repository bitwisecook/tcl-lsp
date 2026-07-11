interp alias {} myset {} set

# Aliased `set` still resolves to the real command, so this loop-carried
# int/string oscillation through the alias is a genuine thunk -> S102.
proc aliased {} {
    myset x 0
    while {1} {
        myset x [expr {$x + 1}]
        myset x [string range $x 0 end]
    }
}

# arr(n) is always int and arr(label) always string -> independent slots, not
# one variable oscillating. Must not fire S100/S101 (FP-SH-13).
proc arrayelems {} {
    set arr(n) 5
    set arr(label) "text"
    incr arr(n)
}

# A numeric/string oscillation seeded by an int entry (`set x 0`): the loop
# body is Numeric on one arm and String on the other. Previously masked to
# OVERDEFINED by type_join; now fires S102 (FP-SH-18).
proc numeric {n} {
    set x 0
    while {$n > 0} {
        if {$n % 2} {
            set x [expr {$x + $n}]
        } else {
            set x "s$x"
        }
        incr n -1
    }
    return $x
}
