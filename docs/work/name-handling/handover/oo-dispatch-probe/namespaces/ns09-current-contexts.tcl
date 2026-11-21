namespace eval ::n {
    proc p {} { namespace current }
    oo::class create C { method m {} { string match ::oo::Obj* [namespace current] } }
}
puts [list [::n::p] [apply {{} {namespace current} ::n}] [[::n::C new] m] [namespace eval ::n::deep {namespace current}] [namespace current]]
