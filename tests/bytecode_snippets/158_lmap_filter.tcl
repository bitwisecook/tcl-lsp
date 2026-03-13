set evens [lmap x {1 2 3 4 5 6} {
    if {$x % 2 == 0} {
        set x
    } else {
        continue
    }
}]
