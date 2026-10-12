proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
namespace eval ::r2286_holder {
    set a [catch {::info -nons commands -all r2286*} av]
    set b [catch {::info -nons commands -all ::r2286_holder::r2286*} bv]
    if {!$a} {set av [lsort $av]}
    if {!$b} {set bv [lsort $bv]}
    list $a $av $b $bv
}
