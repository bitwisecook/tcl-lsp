proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
namespace eval ::r2286_holder {
    set a [catch {::info -nons commands} av]
    set b [catch {::info -nons commands -all} bv]
    foreach slot {av bv} code [list $a $b] {
        if {!$code} {
            set selected {}
            foreach name [set $slot] {if {[string match r2286* $name]} {lappend selected $name}}
            set $slot [lsort $selected]
        }
    }
    list $a $av $b $bv
}
