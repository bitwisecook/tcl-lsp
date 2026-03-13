proc test {args} {
    if {[info exists args]} {
        return "yes"
    }
    return "no"
}
