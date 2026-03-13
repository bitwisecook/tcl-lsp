proc classify {x} {
    switch $x {
        a { return "alpha" }
        b { return "beta" }
        c { return "gamma" }
        default { return "unknown" }
    }
}
