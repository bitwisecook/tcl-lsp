# Setup pairing only; no semantic test cases are run.
set code [catch {
    if {[info patchlevel] ne $::env(PAIRING_EXPECTED_PATCHLEVEL)} {error "unexpected Jim patchlevel"}
    puts "patchlevel=[info patchlevel]"
    puts "executable=[info nameofexecutable]"
    puts "auto_path=$::auto_path"
    puts "support=$::env(PAIRING_JIM_SUPPORT)"
    source $::env(PAIRING_JIM_SUPPORT)
    if {[llength [info commands ::test]] != 1} {error "original Jim test support command missing"}
    if {$::testinfo(numtests) != 0} {error "semantic tests unexpectedly ran"}
    puts "support_command=::test"
    puts "tests_run=$::testinfo(numtests)"
    puts "pairing=accepted"
} result]
if {$code != 0} {
    puts stderr "pairing_error=$result"
    exit 1
}
exit 0
