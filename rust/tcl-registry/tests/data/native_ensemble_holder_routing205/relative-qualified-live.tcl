namespace eval ::P::N {namespace eval q {}}
set result [namespace eval ::P::N {
    namespace ensemble create -command q::E -map {go ::list}
}]
list $result [namespace ensemble exists ::P::N::q::E] [::P::N::q::E go VALUE] [info commands ::q::E]
