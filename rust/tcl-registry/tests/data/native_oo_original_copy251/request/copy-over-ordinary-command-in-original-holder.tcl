if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    ::oo::object create ::r2286_original_oo_source
    ::oo::objdefine ::r2286_original_oo_source method marker {} {return SOURCE}
    namespace eval a: {
        proc p {} {return ORIGINAL_PROC}
        set code [catch {::oo::copy ::r2286_original_oo_source p} result]
        list $code $result [p] [info object isa object p] [::r2286_original_oo_source marker]
    }
}
