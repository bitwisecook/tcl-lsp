if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
foreach {label script} {
    no_operands {lassign}
    literal_no_target {lassign {A B}}
    dynamic_no_target {set probeHead lassign; $probeHead {A B}}
    compiled_no_target {proc probeCompiled {} {lassign {A B}}; probeCompiled}
    malformed_no_target {set probeBad \{; set probeHead lassign; $probeHead $probeBad}
    literal_one_target {lassign {A B} probeFirst}
    empty_no_target {lassign {}}
} {
    set probeCode [catch $script probeResult]
    puts [list RESULT $label $probeCode $probeResult]
}
