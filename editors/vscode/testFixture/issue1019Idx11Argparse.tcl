proc ::argparse {args} {
    if {[llength $args] == 0} { return case0 }
    return [lindex $args 0]
}
puts [argparse]
