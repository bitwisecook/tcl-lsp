proc ::r2286_global {} {return GLOBAL}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
    set code [catch {::info commands -all} result]
    set selected {}
    if {$code == 0} {foreach name $result {if {[string match r2286* $name]} {lappend selected $name}}}
    list $code [lsort $selected]
}
