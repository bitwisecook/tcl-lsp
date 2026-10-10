if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    rename {namespace ensemble} original_namespace_ensemble
    proc {namespace ensemble} args {list GLOBAL $args [uplevel 1 namespace current]}
    namespace eval ::N {namespace ensemble create -map {go ::list}}
}
