if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
set ::probeLog {}
proc probeTrace {name element operation} {
    lappend ::probeLog $operation
    if {[string equal $operation read]} {
        unset ::probeValue
        set ::probeValue 7
    }
}
set ::probeValue 1
set probeSetupCode [catch {
    trace add variable ::probeValue {read unset} probeTrace
} probeSetupResult]
binary scan $probeSetupResult H* probeSetupHex
puts "SETUP $probeSetupCode $probeSetupHex"
if {!$probeSetupCode} {
    set probeCode [catch {
        set probeRead $::probeValue
        list $probeRead $::probeValue $::probeLog [trace info variable ::probeValue]
    } probeResult]
    binary scan $probeResult H* probeResultHex
    puts "OBSERVATION $probeCode $probeResultHex"
}
