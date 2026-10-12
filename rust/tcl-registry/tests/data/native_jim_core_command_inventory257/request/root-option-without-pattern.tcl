proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
set code [catch {::info commands -all} result]
if {$code} {
    list $code $result
} else {
    set selected {}
    foreach name $result {if {[string match r2286* $name]} {lappend selected $name}}
    list $code [lsort $selected]
}
