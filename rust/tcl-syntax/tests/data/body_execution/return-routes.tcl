# Fixed completion routes measured independently on C Tcl and Jim.
proc returnRoute {label args} {
    proc returnInner {} [concat return $args payload]
    proc returnOuter {} {returnInner; return continued}
    set code [catch returnOuter result]
    if {$code == 1} { set result error }
    puts [list $label $code $result]
}
returnRoute plain
returnRoute code-return -code return
returnRoute return-zero -code return -level 0
returnRoute return-two -code return -level 2
returnRoute level-zero -level 0
returnRoute level-two -level 2
returnRoute configured-break -code break
returnRoute custom-option -foo bar
returnRoute invalid-level -level -1
proc rawBreak {} {break}
proc rawContinue {} {continue}
foreach command {rawBreak rawContinue} {
    set code [catch $command result]
    if {$code == 1} {set result error}
    puts [list $command $code $result]
}
foreach value {-4294967295 -2147483649 2147483648 4294967295 4294967296 9223372036854775807 9223372036854775808} {
    proc codeInner {} [list return -code $value payload]
    set code [catch codeInner result]
    if {$result != "payload"} {set result error}
    puts [list $value $code $result]
}
