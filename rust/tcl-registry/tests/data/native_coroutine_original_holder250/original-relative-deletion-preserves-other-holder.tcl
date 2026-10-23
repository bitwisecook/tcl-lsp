if {![llength [info commands coroutine]]} {
    list NOT_APPLICABLE
} else {
    proc ::body {tag} {yield [list $tag [info coroutine]]; return [list DONE $tag]}
    namespace eval a: {set ::first [coroutine p ::body FIRST]}
    namespace eval a {set ::second [coroutine :p ::body SECOND]}
    namespace eval a: {rename p {}}
    set ::firstPresent [namespace eval a: {llength [info commands p]}]
    set ::secondDone [namespace eval a {:p}]
    list $::first $::second $::firstPresent $::secondDone
}
