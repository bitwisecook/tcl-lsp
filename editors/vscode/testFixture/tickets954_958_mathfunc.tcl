namespace eval ::tcl::mathfunc {
    proc Pi {} { return [expr {acos(-1)}] }
    proc deg2rad {deg} { return [expr {$deg * Pi() / 180.0}] }
}
set pi2 [expr {Pi() / 2.0}]
set rad [expr {deg2rad(45)}]
