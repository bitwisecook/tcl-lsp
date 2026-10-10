if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    namespace eval ::N {
        proc go {value} {list DEFAULT $value}
        set made [namespace ensemble create]
    }
    list $::N::made [::N go VALUE] [info args ::N] [lsort [::N -commands]]
}
