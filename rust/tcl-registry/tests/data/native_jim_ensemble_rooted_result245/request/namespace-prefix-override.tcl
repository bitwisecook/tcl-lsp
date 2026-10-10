if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    namespace eval ::Target {proc go {value} {list TARGET $value}}
    namespace eval ::N {
        proc go {value} {list LOCAL $value}
        set ::made [namespace ensemble create -automap ::Target::]
    }
    list $::made [::N go VALUE] [lsort [::N -commands]]
}
