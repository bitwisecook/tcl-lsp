set cases {{} {0} {0 -g 1} {0 -format A -format B} {0 -gmt 0 -timezone :GMT} {bad -gmt bad} {bad -gmt 1 -timezone :GMT} {0 -z X} {0 - X} {0 -format} {9223372036854775808}}
foreach args $cases {set ::errorCode UNSET; set code [catch {eval [linsert $args 0 ::tcl::clock::ParseFormatArgs]} value]; puts [list $args $code $value $::errorCode]}
