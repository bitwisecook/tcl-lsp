if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    ::oo::object create ::source
    ::oo::objdefine ::source method marker {} {return SOURCE}
    namespace eval a: {
        set ::first [::oo::copy ::source p]
        ::oo::objdefine p method marker {} {return FIRST}
    }
    namespace eval a {
        set ::second [::oo::copy ::source :p]
        ::oo::objdefine :p method marker {} {return SECOND}
    }
    list [::source marker] $::first $::second [namespace eval a: {p marker}] [namespace eval a {:p marker}] [namespace eval a: {info object isa object p}] [namespace eval a {info object isa object :p}]
}
