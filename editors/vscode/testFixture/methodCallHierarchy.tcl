oo::class create Greeter {
    method greet {} { return hi }
    method twice {} {
        if {1} {
            my greet
        }
        my greet
    }
}
set g [Greeter new]
