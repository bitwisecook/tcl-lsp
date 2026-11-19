foreach spec [list bogus rw [list [format %c 123]] [format %c 123] [list {}] [list 1234 5678] [list hidden readonly] [list {macintosh type 1} {macintosh creator 2}]] {
 set rc [catch [list glob -nocomplain -types $spec /dev/null] r]
 if {$rc} {puts [list $spec $rc $r $::errorCode]} else {puts [list $spec $rc $r]}
}
foreach args [list [list -nocomplain -types] [list -nocomplain -types bogus -types c /dev/null] [list -types bogus]] {
 set rc [catch [linsert $args 0 glob] r]
 if {$rc} {puts [list $args $rc $r $::errorCode]} else {puts [list $args $rc $r]}
}
