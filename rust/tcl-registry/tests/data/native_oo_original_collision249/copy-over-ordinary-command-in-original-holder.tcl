if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    ::oo::object create ::source
    ::oo::objdefine ::source method marker {} {return SOURCE}
    namespace eval a: {
        proc p {} {return ORIGINAL_PROC}
        set code [catch {::oo::copy ::source p} result]
        list $code $result [p] [info object isa object p] [::source marker]
    }
}
