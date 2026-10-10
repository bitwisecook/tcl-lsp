if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
foreach {label script} {
    literal_array_read {array set a {k OLD}; set a}
    dynamic_array_read {array set a {k OLD}; set probeHead set; $probeHead a}
    procedure_array_read {proc probeArrayRead {} {array set a {k OLD}; set a}; probeArrayRead}
    literal_scalar_element {set b SCALAR; set b(k)}
    dynamic_scalar_element {set b SCALAR; set probeHead set; $probeHead b(k)}
    procedure_scalar_element {proc probeScalarRead {} {set b SCALAR; set b(k)}; probeScalarRead}
} {
    set probeCode [catch $script probeResult]
    puts [list RESULT $label $probeCode $probeResult]
}
