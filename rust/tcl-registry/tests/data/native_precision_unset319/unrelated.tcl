set ::tcl_precision 12
namespace eval n {
 variable tcl_precision LOCAL
 list [info exists tcl_precision] $tcl_precision [info exists ::tcl_precision] $::tcl_precision
}
