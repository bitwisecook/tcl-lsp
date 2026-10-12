if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    namespace eval ::N {
        proc {namespace ensemble} args {list LOCAL $args [uplevel 1 namespace current]}
        namespace ensemble create -map {go ::list}
    }
}
