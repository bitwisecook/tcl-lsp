puts "VERSION|[info patchlevel]"
proc record {label source} {
    set status [catch {uplevel 1 $source} value]
    binary scan $value H* hex
    puts "$label|$status|$hex"
}
record initial-slot {info commands ::tcl::mathfunc::Pi}
record initial-call {expr {Pi()}}
record namespace-control {namespace eval ::tcl::mathfunc {}}
record install-control {proc ::tcl::mathfunc::Pi {} {return 17}}
record installed-slot {info commands ::tcl::mathfunc::Pi}
record installed-call {expr {Pi()}}
