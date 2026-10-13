proc lost {} {return PLAIN}
set ::auto_path [linsert $::auto_path 0 {/tmp/name-spec-probe-studio-20261013/shared-family-controls/package}]
if {![catch {::info tclversion} provider]} {package ifneeded demo 1.0 [list source {/tmp/name-spec-probe-studio-20261013/shared-family-controls/package/demo.tcl}]}
namespace eval ::__tcl_name_query {}
proc ::__tcl_name_query::family {} {
    if {![catch {::info tclversion} version] && [lsearch -exact {8.4 8.5 8.6 9.0 9.1} $version] >= 0} {
        return c
    }
    if {![catch {::info version} version] && $version eq "0.84"} {return jim}
    error "name query has no selected interpreter grammar"
}
namespace eval ::__tcl_command_inventory {}
proc ::__tcl_command_inventory::in_root {excluded} {
    if {[::__tcl_name_query::family] eq "jim"} {
        set found {}
        foreach word [uplevel #0 {::info -nons commands}] {
            set private 0
            foreach ns [concat $excluded {::__tcl_command_inventory ::__tcl_name_query}] {
                set prefix [string range $ns 2 end]::
                if {[string first $prefix $word] == 0} {set private 1; break}
            }
            if {$private} continue
            if {[string first "\0" $word] >= 0 || ![uplevel #0 [list ::exists -command $word]]} {
                error "command inventory has no original callable root word"
            }
            lappend found $word
        }
        return [lsort -unique $found]
    }
    set found {}
    set pending [list ::]
    while {[llength $pending]} {
        set ns [lindex $pending 0]
        set pending [lrange $pending 1 end]
        if {[lsearch -exact $excluded $ns] >= 0 || [lsearch -exact {::__tcl_command_inventory ::__tcl_name_query} $ns] >= 0} continue
        if {[string first "\0" $ns] >= 0 || [string range $ns 0 1] ne "::" || [string first ":::" $ns] >= 0 || ($ns ne "::" && [string index $ns end] eq ":")} {
            error "command inventory has no unambiguous namespace address"
        }
        foreach child [::namespace children $ns] {lappend pending $child}
        foreach word [::namespace inscope $ns {::info commands}] {
            if {[string first "\0" $word] >= 0 || [string first :: $word] >= 0} {
                error "command inventory has no original simple command word"
            }
            set selected [::namespace inscope $ns [list ::namespace which -command $word]]
            set expected [expr {$ns eq "::" ? "::$word" : "${ns}::$word"}]
            if {$selected ne $expected} continue
            if {$ns eq "::"} {
                set operand $word
            } else {
                if {[string index $word 0] eq ":"} {error "command inventory has no callable root operand"}
                set operand ${ns}::$word
                if {[uplevel #0 [list ::namespace which -command $operand]] ne $selected} {
                    error "command inventory root lookup disagrees with its original context"
                }
            }
            lappend found $operand
        }
    }
    return [lsort -unique $found]
}
namespace eval ::__spec_probe {}
proc ::__spec_probe::say {kind a b} {
    set a [string map [list \\ \\\\ \t \\t \n \\n \r \\r] $a]
    set b [string map [list \\ \\\\ \t \\t \n \\n \r \\r] $b]
    puts "\nSPEC-PROBE\t$kind\t$a\t$b"
}
proc ::__spec_probe::commands {} {
    return [::__tcl_command_inventory::in_root {::__spec_probe}]
}
catch {package require __spec_probe_warmup}
array set ::__spec_probe::before {}
if {[catch {::__spec_probe::commands} inventory]} {
    ::__spec_probe::say failure {} "command inventory unavailable: $inventory"
    ::__spec_probe::say done 0 {}
    exit 0
}
foreach name $inventory {set ::__spec_probe::before($name) 1}

if {[catch {package require demo} version]} {
::__spec_probe::say failure {} $version
::__spec_probe::say done 0 {}
exit 0
}
::__spec_probe::say provided {} $version
set ::__spec_probe::count 0
if {[catch {::__spec_probe::commands} inventory]} {
    ::__spec_probe::say failure {} "command inventory unavailable: $inventory"
    ::__spec_probe::say done 0 {}
    exit 0
}
foreach name $inventory {
    if {![info exists ::__spec_probe::before($name)]} {
        incr ::__spec_probe::count
        ::__spec_probe::say added $name {}
    }
}
::__spec_probe::say done $::__spec_probe::count {}
