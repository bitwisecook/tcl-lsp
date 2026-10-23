set ::tcl_precision 12
proc linked {} {
 upvar #0 ::tcl_precision p
 unset p
 set aliasExists [info exists p]
 set rootExists [info exists ::tcl_precision]
 set aliasCode [catch {set p} aliasValue]
 set rootCode [catch {set ::tcl_precision} rootValue]
 set reportedCode [catch {set ::p} reportedValue]
 list $aliasExists $rootExists $aliasCode $aliasValue $rootCode $rootValue $reportedCode $reportedValue
}
linked
