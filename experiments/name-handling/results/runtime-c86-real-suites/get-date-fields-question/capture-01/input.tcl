puts [list RELEASE [info patchlevel] AVAILABLE [llength [info commands ::tcl::clock::GetDateFields]]]
proc emit {label script} {
    set code [catch {uplevel #0 $script} result]
    set ec {}
    if {$code == 1} {set ec $::errorCode}
    puts [list ROW $label CODE $code RESULT $result ERRORCODE $ec]
}
emit arity0 {::tcl::clock::GetDateFields }
emit arity2 {::tcl::clock::GetDateFields 0 {}}
emit arity4 {::tcl::clock::GetDateFields 0 {} 2361222 extra}
emit epoch {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} 2361222}
emit negative {::tcl::clock::GetDateFields -1 {{-9223372036854775808 0 0 GMT}} 2361222}
emit leap2000 {::tcl::clock::GetDateFields 951782400 {{-9223372036854775808 0 0 GMT}} 2361222}
emit iso2016 {::tcl::clock::GetDateFields 1451606400 {{-9223372036854775808 0 0 GMT}} 2361222}
emit julian-calendar {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} 2147483647}
emit gregorian-forced {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} 0}
emit changeover-before {::tcl::clock::GetDateFields -12219292801 {{-9223372036854775808 0 0 GMT}} 2299161}
emit changeover-at {::tcl::clock::GetDateFields -12219292800 {{-9223372036854775808 0 0 GMT}} 2299161}
emit before-first {::tcl::clock::GetDateFields 9 {{10 100 0 A} {100 200 1 B}} 2361222}
emit transition-at {::tcl::clock::GetDateFields 100 {{10 100 0 A} {100 200 1 B}} 2361222}
emit transition-between {::tcl::clock::GetDateFields 99 {{10 100 0 A} {100 200 1 B}} 2361222}
emit seconds-bad {::tcl::clock::GetDateFields bad {{-9223372036854775808 0 0 GMT}} 2361222}
emit changeover-bad {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} bad}
emit both-bad {::tcl::clock::GetDateFields bad {bad} bad}
emit seconds-unsigned {::tcl::clock::GetDateFields 9223372036854775808 {{-9223372036854775808 0 0 GMT}} 2361222}
emit unsigned-before-badchange {::tcl::clock::GetDateFields 9223372036854775808 {{-9223372036854775808 0 0 GMT}} bad}
emit seconds-too-large {::tcl::clock::GetDateFields 18446744073709551616 {{-9223372036854775808 0 0 GMT}} 2361222}
emit changeover-unsigned {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} 4294967295}
emit changeover-too-large {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT}} 4294967296}
emit outer-list-bad {::tcl::clock::GetDateFields 0 "\{" 2361222}
emit transition-bad {::tcl::clock::GetDateFields 0 {{bad 0 0 GMT}} 2361222}
emit offset-bad {::tcl::clock::GetDateFields 0 {{-9223372036854775808 bad 0 GMT}} 2361222}
emit offset-too-large {::tcl::clock::GetDateFields 0 {{-9223372036854775808 4294967296 0 GMT}} 2361222}
emit ignored-dst {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 bad GMT}} 2361222}
emit ignored-extra {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 GMT ignored}} 2361222}
emit unvisited-bad-row {::tcl::clock::GetDateFields -1 {{0 0 0 GMT} bad} 2361222}
emit empty-name {::tcl::clock::GetDateFields 0 {{-9223372036854775808 0 0 {}}} 2361222}
emit empty-tzdata {::tcl::clock::GetDateFields 0 {} 2361222}
emit c9-zone-name {::tcl::clock::GetDateFields 0 :GMT 2361222}
set ::effectlog {}
proc watch {name element operation} {lappend ::effectlog $name}
set ::tick bad
set ::zone bad
set ::change bad
foreach var {::tick ::zone ::change} {trace variable $var r watch}
emit read-effects {::tcl::clock::GetDateFields $::tick $::zone $::change}
puts [list READ_EFFECTS $::effectlog]
foreach var {::tick ::zone ::change} {trace vdelete $var r watch}
