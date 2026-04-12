# TclOO classes used by new LSP feature tests.
oo::class create Animal {
    method speak {} { return "..." }
    method greet {} { my speak }
}

oo::class create Dog {
    superclass Animal
    method speak {} { return "woof" }
}

oo::class create Cat {
    superclass Animal
    method speak {} { return "meow" }
}

proc make_dog {} {
    set d [Dog new]
    return $d
}

proc factorial {n} {
    if {$n <= 1} { return 1 }
    return [expr {$n * [factorial [expr {$n - 1}]]}]
}
