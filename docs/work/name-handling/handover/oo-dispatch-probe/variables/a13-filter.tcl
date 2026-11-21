oo::class create C {
    method f args { return "f:[next {*}$args]" }
    method m {} { return m }
    filter f
}
puts [[C new] m]
