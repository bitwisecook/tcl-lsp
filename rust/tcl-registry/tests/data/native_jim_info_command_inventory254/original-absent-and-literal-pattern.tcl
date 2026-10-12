proc ::r2286_global {} {return GLOBAL}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
    set selected {}
    foreach name [info commands] {if {[string match r2286* $name]} {lappend selected $name}}
    list [lsort $selected] [lsort [info commands r2286*]]
}
