namespace eval ::P::N {
    proc worker {} {
        namespace delete ::P
        namespace eval ::P::N {variable marker NEW}
        set result [namespace ensemble create -map {go ::list}]
        list $result [namespace ensemble exists ::P::N] [info commands ::P::N] \
            [catch {::P::N go VALUE} code] $code
    }
}
set ::observed [::P::N::worker]
