if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    ::oo::object create ::r2286_original_oo_source
    ::oo::objdefine ::r2286_original_oo_source method marker {} {return SOURCE}
    namespace eval a: {
        set ::first [::oo::copy ::r2286_original_oo_source p]
        ::oo::objdefine p method marker {} {return FIRST}
    }
    namespace eval a {
        set ::second [::oo::copy ::r2286_original_oo_source :p]
        ::oo::objdefine :p method marker {} {return SECOND}
    }
    list [::r2286_original_oo_source marker] $::first $::second [namespace eval a: {p marker}] [namespace eval a {:p marker}] [namespace eval a: {info object isa object p}] [namespace eval a {info object isa object :p}]
}
