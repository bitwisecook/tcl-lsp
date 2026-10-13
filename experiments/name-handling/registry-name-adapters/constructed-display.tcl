namespace eval ::tcl::mathfunc {
    proc :f {} {return colon-simple}
    puts [list local-colon [namespace current] [info procs :f] [:f]]
}
namespace eval ::tcl::mathfunc: {
    proc f {} {return colon-holder}
    puts [list holder-colon [namespace current] [info procs f] [f]]
}
set displayedA "::tcl::mathfunc:::f"
set displayedB "::tcl::mathfunc:::f"
puts [list equal-display [string equal $displayedA $displayedB]]
set code [catch {eval [list $displayedA]} value]
puts [list written-relookup $code $value]
