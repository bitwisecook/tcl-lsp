if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    namespace eval a: {
        set ::created [::oo::object create p]
        ::oo::objdefine p method marker {} {return FIRST}
        list $::created [namespace current] [info object isa object p] [p marker]
    }
}
