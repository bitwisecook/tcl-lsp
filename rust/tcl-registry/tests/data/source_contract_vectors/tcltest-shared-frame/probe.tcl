package require tcltest
namespace import ::tcltest::test

proc direct {} {
    test direct missing-read -setup {set prepared 1} \
        -body {list $prepared $absent} \
        -cleanup {set ::directCleanup $prepared} \
        -returnCodes error -result {can't read "absent": no such variable}
    list $::directCleanup [info exists absent]
}
puts [list direct [direct]]

interp alias {} captured {} ::tcltest::test captured missing-read
proc aliasControl {} {
    captured -setup {set prepared 1} \
        -body {list $prepared $absent} \
        -cleanup {set ::aliasCleanup $prepared} \
        -returnCodes error -result {can't read "absent": no such variable}
    list $::aliasCleanup [info exists absent]
}
puts [list captured-alias [aliasControl]]

proc stoppedSetup {} {
    set ::bodyEntered 0
    test stopped setup-error -setup {set prepared 1; error stop} \
        -body {set ::bodyEntered 1; list $unreached} \
        -cleanup {set ::stoppedCleanup $prepared} -result {}
    list $::bodyEntered $::stoppedCleanup
}
puts [list stopped-setup [stoppedSetup]]

proc opaqueResult {} {
    test opaque result-data -body {list {[puts $data_only]}} \
        -result {{[puts $data_only]}}
    info exists data_only
}
puts [list opaque-result [opaqueResult]]
puts [list counts $::tcltest::numTests(Total) \
    $::tcltest::numTests(Passed) $::tcltest::numTests(Failed)]
