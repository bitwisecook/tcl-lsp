set events {}
proc observe {name1 name2 op} {
    lappend ::events $op
    array set ::a {x value}
}
set registration [catch {trace add variable a array observe} registration_result]
puts [list registration $registration $registration_result]
if {$registration == 0} {
set code [catch {array names a -bad x} result]
set error_code {}
if {$code != 0 && [info exists ::errorCode]} {set error_code $::errorCode}
puts [list $code $result $error_code]
    puts [list events $events]
}
