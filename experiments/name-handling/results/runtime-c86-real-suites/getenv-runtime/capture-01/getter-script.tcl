set worker ::tcl::clock::getenv
foreach args {{} {NH_CLOCK_GETENV_TEST} {NH_CLOCK_GETENV_MISSING} {NH_CLOCK_GETENV_TEST extra} {{}}} {
    set ::errorCode UNSET
    set code [catch {eval [linsert $args 0 $worker]} value]
    puts [list $args $code $value $::errorCode]
}
if {[llength [info commands $worker]]} {
    set ::readCalls 0
    proc ::readTrap {a b op} {incr ::readCalls; error SHOULD_NOT_READ_ENV_ARRAY}
    trace add variable ::env(NH_CLOCK_GETENV_TEST) read ::readTrap
    set value [$worker NH_CLOCK_GETENV_TEST]
    puts [list read-trace $value $::readCalls]
    trace remove variable ::env(NH_CLOCK_GETENV_TEST) read ::readTrap
    set ::env(NH_CLOCK_GETENV_TEST) script-value
    puts [list after-element-write [$worker NH_CLOCK_GETENV_TEST]]
    unset ::env(NH_CLOCK_GETENV_TEST)
    puts [list after-element-unset [$worker NH_CLOCK_GETENV_TEST]]
    set ::env(NH_CLOCK_GETENV_TEST) before-array-unset
    unset ::env
    set ::env(NH_CLOCK_GETENV_TEST) local-only
    puts [list after-array-replacement [$worker NH_CLOCK_GETENV_TEST] $::env(NH_CLOCK_GETENV_TEST)]
}
