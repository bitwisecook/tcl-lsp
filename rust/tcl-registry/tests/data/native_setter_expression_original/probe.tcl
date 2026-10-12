puts [list version [info patchlevel]]
proc reportSetterObservation {label script} {
    set status [catch {uplevel #0 $script} result]
    puts [list OBSERVATION $label $status $result]
}
reportSetterObservation builtin {
    set input 3
    set output [expr {$input + 1}]
    set output
}
reportSetterObservation alias_expression {
    interp alias {} calc {} expr
    set output [calc {$input + 1}]
    set output
}
reportSetterObservation shadowed_expression {
    proc expr args {return VALUE}
    set output [expr {$missing}]
    set output
}
reportSetterObservation alias_setter {
    interp alias {} setter {} set
    setter output [expr {$missing}]
    set output
}
