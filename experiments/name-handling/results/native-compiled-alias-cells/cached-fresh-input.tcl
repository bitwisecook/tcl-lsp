set seen {}
proc observe {tag old new op} {
    set fresh ::P
    append fresh ::C
    lappend ::seen [list $tag [namespace exists ::P::C] [namespace exists $fresh]]
}
namespace eval ::P {
    namespace ensemble create -command ::PE
    namespace eval C {namespace ensemble create -command ::CE}
}
trace add command ::PE delete [list observe parent]
trace add command ::CE delete [list observe child]
namespace delete ::P
puts [list OBS $seen]
