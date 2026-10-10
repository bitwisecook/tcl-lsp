set rows {}
set ::tcl_precision 12
set initial [list [info exists ::tcl_precision] $::tcl_precision]
unset ::tcl_precision
lappend rows [list root [info exists ::tcl_precision] $::tcl_precision]
proc linked {} {
 upvar #0 ::tcl_precision p
 unset p
 list [info exists p] [info exists ::tcl_precision] $::tcl_precision [catch {set p} message] $message
}
lappend rows [list linked [linked]]
rename linked {}
namespace eval n {
 variable tcl_precision LOCAL
 set result [list [info exists tcl_precision] $tcl_precision]
}
lappend rows [list unrelated $::n::result]
set rows
