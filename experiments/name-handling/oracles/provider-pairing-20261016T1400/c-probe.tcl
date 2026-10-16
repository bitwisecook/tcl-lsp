# Setup pairing only; no semantic test cases are run.
set code [catch {
    set expected $::env(PAIRING_EXPECTED_PATCHLEVEL)
    set library $::env(TCL_LIBRARY)
    if {[info patchlevel] ne $expected} {error "unexpected patchlevel"}
    if {[file normalize [info library]] ne [file normalize $library]} {error "unexpected script library"}
    puts "patchlevel=[info patchlevel]"
    puts "executable=[info nameofexecutable]"
    puts "library=[info library]"
    puts "init=[file join [info library] init.tcl]"
    set supplied [package require tcltest]
    if {$supplied ne $::env(PAIRING_EXPECTED_TCLTEST)} {error "unexpected tcltest version"}
    set loader [package ifneeded tcltest $supplied]
    set expectedSupport [file join [info library] tcltest tcltest.tcl]
    if {[string first $expectedSupport $loader] < 0} {error "tcltest loader does not name matching source tree"}
    if {[llength [info commands ::tcltest::test]] != 1} {error "tcltest command missing"}
    puts "tcltest_version=$supplied"
    puts "tcltest_loader=$loader"
    puts "tcltest_command=::tcltest::test"
    puts "loaded=[info loaded]"
    puts "pairing=accepted"
} result]
if {$code != 0} {
    puts stderr "pairing_error=$result"
    puts stderr $::errorInfo
    exit 1
}
exit 0
