set {} EMPTY
set {$name} LITERAL
puts [list QUIET_EXISTENCE [info exists {}] [info exists {$name}] [set {}] [set {$name}]]
set watched ORIGINAL
set callback_count 0
proc original_read_callback {args} {incr ::callback_count}
set trace_code [catch {trace variable watched r original_read_callback} trace_error]
puts [list TRACE_INSTALL $trace_code $trace_error]
if {$trace_code == 0} {
    puts [list WATCHED_EXISTENCE [info exists watched] $callback_count]
}
