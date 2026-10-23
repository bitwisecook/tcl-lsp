if {[catch {info commands -all r2286_key*}]} {
    list NOT_APPLICABLE
} else {
    proc ::r2286_key {} {return FIRST}
    rename r2286_key :::r2286_key_moved
    set moved [list [lsort [info commands -all r2286_key*]] [r2286_key_moved]]
    proc r2286_key_moved {} {return SECOND}
    set replaced [list [lsort [info commands -all r2286_key*]] [r2286_key_moved]]
    rename r2286_key_moved {}
    proc r2286_key_moved {} {return FRESH}
    set fresh [list [lsort [info commands -all r2286_key*]] [r2286_key_moved]]
    list $moved $replaced $fresh
}
