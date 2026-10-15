puts [list PATCHLEVEL [info patchlevel]]
set word [binary format H* 22686f73745b736574203a3a68696e7420315d22]
set ::hint 0
set script "list $word"
set code [catch {eval $script} value]
set bodyword [format "{%s}" $script]
set bodycode [catch {eval "list $bodyword"} body]
set entered -1
set result {}
if {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}
puts [list OLD_SUBSTITUTING WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]
set word [binary format H* 22686f73745c5b736574203a3a68696e7420315c5d22]
set ::hint 0
set script "list $word"
set code [catch {eval $script} value]
set bodyword [format "{%s}" $script]
set bodycode [catch {eval "list $bodyword"} body]
set entered -1
set result {}
if {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}
puts [list LITERAL_SUBSTITUTION WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]
set word [binary format H* 225c22686f73745c2222]
set ::hint 0
set script "list $word"
set code [catch {eval $script} value]
set bodyword [format "{%s}" $script]
set bodycode [catch {eval "list $bodyword"} body]
set entered -1
set result {}
if {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}
puts [list QUOTED_DATA WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]
set word [binary format H* 227d22]
set ::hint 0
set script "list $word"
set code [catch {eval $script} value]
set bodyword [format "{%s}" $script]
set bodycode [catch {eval "list $bodyword"} body]
set entered -1
set result {}
if {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}
puts [list QUOTED_UNBALANCED_BRACE WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]
set word [binary format H* 5c7d]
set ::hint 0
set script "list $word"
set code [catch {eval $script} value]
set bodyword [format "{%s}" $script]
set bodycode [catch {eval "list $bodyword"} body]
set entered -1
set result {}
if {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}
puts [list ESCAPED_BRACE WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]
set word [binary format H* 225c246c69746572616c22]
set ::hint 0
set script "list $word"
set code [catch {eval $script} value]
set bodyword [format "{%s}" $script]
set bodycode [catch {eval "list $bodyword"} body]
set entered -1
set result {}
if {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}
puts [list LITERAL_SIGIL WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]
