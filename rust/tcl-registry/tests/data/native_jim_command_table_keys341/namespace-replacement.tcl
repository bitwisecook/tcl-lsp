if {[catch {info commands -all r2286_key_holder::*}]} {
    list NOT_APPLICABLE
} else {
    namespace eval r2286_key_holder {proc local {} {return FIRST}}
    set first [list [lsort [info commands -all r2286_key_holder::local]] [r2286_key_holder::local]]
    namespace eval r2286_key_holder {proc ::r2286_key_holder::local {} {return SECOND}}
    set replaced [list [lsort [info commands -all r2286_key_holder::local]] [r2286_key_holder::local]]
    list $first $replaced
}
