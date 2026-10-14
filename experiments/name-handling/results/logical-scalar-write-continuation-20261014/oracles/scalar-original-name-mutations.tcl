puts [list PROVIDER [info patchlevel]]
foreach name [list {} {$counter}] {
 set $name 8
 set code [catch [list incr $name] value]
 puts [list ORIGINAL_INCREMENT $name $code $value [set $name]]
}
unset {}
set code [catch {incr {}} value]
puts [list MISSING_EMPTY_INCREMENT $code $value [info exists {}]]
set counter 9
incr counter
puts [list PRIOR_LITERAL_CHANGED [set counter]]
set trace_count 0
proc observe {args} {global trace_count; incr trace_count}
set traced 8
set code [catch {trace variable traced rw observe} value]
puts [list TRACE_REGISTRATION $code $value]
if {$code == 0} {
 incr traced
 puts [list TRACE_CALLBACKS $trace_count]
}
