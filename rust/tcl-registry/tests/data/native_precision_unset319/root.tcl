set ::tcl_precision 12
unset ::tcl_precision
set exists [info exists ::tcl_precision]
set code [catch {set ::tcl_precision} value]
list $exists $code $value
