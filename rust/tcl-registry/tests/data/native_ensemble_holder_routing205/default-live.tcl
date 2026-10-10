set result [namespace eval ::P::N {
    namespace ensemble create -map {go ::list}
}]
list $result [namespace ensemble exists ::P::N] [::P::N go VALUE]
