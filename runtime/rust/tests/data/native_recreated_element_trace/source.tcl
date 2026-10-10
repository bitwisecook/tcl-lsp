if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
set ::a(k) 10
set ::probeLog {}
proc probeNew args {
    lappend ::probeLog READ
}
proc probeOld args {
    unset ::a
    set ::a(k) 100
    trace add variable ::a(k) read probeNew
    set ::probeSeen [set ::a(k)]
}
set probeSetupCode [catch {
    trace add variable ::a(k) read probeOld
} probeSetupResult]
binary scan $probeSetupResult H* probeSetupHex
puts "SETUP $probeSetupCode $probeSetupHex"
if {!$probeSetupCode} {
    set probeCode [catch {incr ::a(k)} probeResult]
    trace remove variable ::a(k) read probeNew
    set probeObservation [list $probeCode $probeResult $::probeSeen $::probeLog $::a(k)]
    binary scan $probeObservation H* probeHex
    puts "OBSERVATION $probeHex"
}
