if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    namespace eval a: {
        proc p {} {return ORIGINAL_PROC}
        set code [catch {::oo::object create p} result]
        list $code $result [p] [info object isa object p]
    }
}
