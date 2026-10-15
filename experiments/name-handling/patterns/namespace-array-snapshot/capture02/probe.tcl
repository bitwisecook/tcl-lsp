puts [list provider [info patchlevel] [info library]]
namespace eval ::__tcl_name_snapshot {}
proc ::__tcl_name_snapshot::namespace_address {ns} {
    if {[string first "\0" $ns] >= 0 || [string range $ns 0 1] ne "::" || [string first ":::" $ns] >= 0 || ($ns ne "::" && [string range $ns end-1 end] eq "::")} {
        error "namespace report has no unambiguous snapshot address"
    }
    return $ns
}
proc ::__tcl_name_snapshot::simple_operand {name} {
    if {[string first "\0" $name] >= 0 || [string first :: $name] >= 0 || ([string index $name end] eq ")" && [string first ( $name] >= 0)} {
        error "variable report has no original simple snapshot operand"
    }
    return $name
}
proc ::__tcl_name_snapshot::coordinate {coordinate} {
    if {[llength $coordinate] != 2} {error "variable snapshot requires namespace and original simple word"}
    foreach {ns name} $coordinate break
    ::__tcl_name_snapshot::namespace_address $ns
    ::__tcl_name_snapshot::simple_operand $name
    return $coordinate
}
proc ::__tcl_name_snapshot::variables {excluded} {
    set found {}
    set pending [list ::]
    while {[llength $pending]} {
        set ns [lindex $pending 0]
        set pending [lrange $pending 1 end]
        if {[lsearch -exact $excluded $ns] >= 0 || $ns eq "::__tcl_name_snapshot"} continue
        ::__tcl_name_snapshot::namespace_address $ns
        foreach child [::namespace children $ns] {lappend pending $child}
        foreach name [::namespace inscope $ns {::info vars}] {
            ::__tcl_name_snapshot::simple_operand $name
            set selected [::namespace inscope $ns [list ::namespace which -variable $name]]
            set expected [expr {$ns eq "::" ? "::$name" : "${ns}::$name"}]
            if {$selected ne $expected} continue
            lappend found [list $ns $name]
        }
    }
    return [lsort -unique $found]
}
proc ::__tcl_name_snapshot::call {coordinate prefix {suffix {}}} {
    foreach {ns name} [::__tcl_name_snapshot::coordinate $coordinate] break
    if {[string range [lindex $prefix 0] 0 1] ne "::"} {error "snapshot operation requires an explicit global command"}
    return [::namespace inscope $ns [concat $prefix [list $name] $suffix]]
}
proc ::__tcl_name_snapshot::element_value {coordinate key} {
    foreach {ns name} [::__tcl_name_snapshot::coordinate $coordinate] break
    return [::namespace inscope $ns [list ::set ${name}($key)]]
}
proc ::__tcl_name_snapshot::shown {coordinate} {
    foreach {ns name} [::__tcl_name_snapshot::coordinate $coordinate] break
    if {$ns eq "::"} {return $name}
    return ${ns}::$name
}

proc observation {label command} {
    set status [catch {uplevel 1 $command} value]
    puts [list $label $status $value]
}
namespace eval ::snapshot_probe {
    array set plain {k PLAIN}
    array set {a(} {k ROOT_WITH_OPEN_PAREN}
    array set a {{(k} OTHER_ARRAY}
    set ordinary [list ::snapshot_probe plain]
    set unusual [list ::snapshot_probe {a(}]
    observation ordinary_array_exists [list ::__tcl_name_snapshot::call $ordinary {::array exists}]
    observation ordinary_actual_array [list ::array get plain]
    observation ordinary_helper_value [list ::__tcl_name_snapshot::element_value $ordinary k]
    observation unusual_simple_operand [list ::__tcl_name_snapshot::simple_operand {a(}]
    observation unusual_array_exists [list ::__tcl_name_snapshot::call $unusual {::array exists}]
    observation unusual_actual_array [list ::array get {a(}]
    observation competing_actual_array [list ::array get a]
    observation unusual_helper_value [list ::__tcl_name_snapshot::element_value $unusual k]
}
