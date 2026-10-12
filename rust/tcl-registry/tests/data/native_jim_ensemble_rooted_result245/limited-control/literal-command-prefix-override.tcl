if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    proc {target go} {value} {list PREFIX $value}
    namespace eval ::N {set made [namespace ensemble create -automap {target }]}
    list $::N::made [::N go VALUE] [lsort [::N -commands]]
}
