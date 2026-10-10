if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    namespace eval a: {
        set ::first [::oo::object create p]
        ::oo::objdefine p method marker {} {return FIRST}
    }
    namespace eval a {
        set ::second [::oo::object create :p]
        ::oo::objdefine :p method marker {} {return SECOND}
    }
    list $::first $::second [namespace eval a: {p marker}] [namespace eval a {:p marker}] [namespace eval a: {info object isa object p}] [namespace eval a {info object isa object :p}]
}
