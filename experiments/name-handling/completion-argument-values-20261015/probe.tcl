puts [list PATCHLEVEL [info patchlevel]]
proc capture {args} {return $args}
set code [catch {capture is # comment} result]
puts [list MID_COMMAND_HASH $code [llength $result] $result]
set code [catch {capture is ; # comment
} result]
puts [list SEPARATED_COMMENT $code [llength $result] $result]
if {[llength [info commands ::oo::class]] == 0} {
    puts {TCLOO unavailable}
} else {
    ::oo::class create C {}
    set code [catch {C "create" object} result]
    puts [list QUOTED_SELECTOR $code [llength [info commands object]]]
}
