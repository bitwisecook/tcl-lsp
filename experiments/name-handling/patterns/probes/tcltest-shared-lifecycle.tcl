package require tcltest
namespace eval ::nh {
    namespace import ::tcltest::*
    set observed {}
    test name-corpus-1 {} -setup {
        set payload {a b}
    } -body {
        lappend observed [set payload]
    } -cleanup {
        lappend observed [set payload]
        unset payload
    } -result {{a b}}
}
list $::nh::observed [info exists ::nh::payload]

