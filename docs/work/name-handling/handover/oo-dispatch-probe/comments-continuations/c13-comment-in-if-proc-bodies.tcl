proc p {x} {
    # leading comment
    if {$x} {
        # comment in if body
        return yes ;# trailing
    } else {
        # comment in else
        return no
    }
}
puts [p 1][p 0]
