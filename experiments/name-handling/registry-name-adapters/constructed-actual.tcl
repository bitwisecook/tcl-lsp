namespace eval ::tcl::mathfunc {
    proc :f {} {return colon-simple}
    set ::displayedA [format "%s::%s" [namespace current] :f]
    puts [list local-colon [namespace current] [info procs :f] [:f]]
}
namespace eval ::tcl::mathfunc: {
    proc f {} {return colon-holder}
    set ::displayedB [format "%s::%s" [namespace current] f]
    puts [list holder-colon [namespace current] [info procs f] [f]]
}
puts [list equal-display [string equal $displayedA $displayedB]]
set code [catch {eval [list $displayedA]} value]
puts [list written-relookup $code $value]
