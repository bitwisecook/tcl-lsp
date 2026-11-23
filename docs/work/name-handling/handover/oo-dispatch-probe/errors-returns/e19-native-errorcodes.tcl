# errorCode set by native commands, as trap would see it.
foreach s {{expr {1/0}} {set nosuchvar} {nosuchcmd} {lindex} {expr {"a"+1}} {open /no/such/file} {incr x y}} {
  catch $s m; puts "[list $s]: $::errorCode"
}
