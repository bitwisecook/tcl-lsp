puts [list RELEASE [info patchlevel] AVAILABLE [llength [info commands ::tcl::clock::GetDateFields]]]
puts [list REAL_CLOCK_WARMUP [clock format 0 -gmt 1 -format %Y]]
proc emit {label script} {
    set code [catch {uplevel #0 $script} result]
    set ec {}
    if {$code == 1} {set ec $::errorCode}
    puts [list ROW $label CODE $code RESULT $result ERRORCODE $ec]
}
emit named-gmt {::tcl::clock::GetDateFields 0 :GMT 2361222}
emit c86-table-is-not-zone {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} 2361222}
emit unsigned-before-badchange {::tcl::clock::GetDateFields 9223372036854775808 :GMT bad}
