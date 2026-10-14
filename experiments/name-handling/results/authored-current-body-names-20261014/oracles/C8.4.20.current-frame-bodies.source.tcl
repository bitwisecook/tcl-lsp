puts [list version [info patchlevel]]
proc current_frame_body_probe {} {
    set x 3
    set y UNCHANGED
    eval {if {$x > 1} {set y 2}}
    puts [list eval_x $x eval_y $y]
    uplevel 0 {set x 2}
    puts [list uplevel_zero_x $x]
    eval {set {$literal} VALUE}
    puts [list literal_sigil_name [info exists {$literal}] literal_sigil_value [set {$literal}]]
    eval {set {} EMPTY}
    puts [list empty_name [info exists {}] empty_value [set {}]]
}
current_frame_body_probe
