proc LocalReceiptP {} {}; namespace eval LocalReceiptScope {proc LocalReceiptP {} {}}
set force_projection 1
rename namespace actual_namespace
proc namespace args {
 if {[lindex $args 0] eq "canon" && [llength $args] == 3 && $::force_projection} {
  set ::force_projection 0
  return ::[actual_namespace canon {*}[lrange $args 1 end]]
 }
 tailcall actual_namespace {*}$args
}
