if {[catch {info commands -all r2286_key}]} {
    list NOT_APPLICABLE
} else {
    proc ::r2286_key {} {return FIRST}
    set first [list [lsort [info commands -all r2286_key]] [r2286_key]]
    proc r2286_key {} {return SECOND}
    set replaced [list [lsort [info commands -all r2286_key]] [r2286_key]]
    alias :::r2286_key list ALIAS
    set aliased [list [lsort [info commands -all r2286_key]] [r2286_key]]
    proc r2286_key {} {return LAST}
    set final [list [lsort [info commands -all r2286_key]] [r2286_key]]
    list $first $replaced $aliased $final
}
