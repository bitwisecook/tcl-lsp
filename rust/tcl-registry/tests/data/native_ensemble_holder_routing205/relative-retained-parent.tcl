namespace eval ::P::N {
    proc worker {} {
        namespace delete ::P
        namespace eval ::P::N {variable marker NEW}
        set result [namespace ensemble create -command E -map {go ::list}]
        list $result [namespace ensemble exists ::P::N::E] [info commands ::P::N::E] \
            [catch {::P::N::E go VALUE} code] $code
    }
}
set ::observed [::P::N::worker]
