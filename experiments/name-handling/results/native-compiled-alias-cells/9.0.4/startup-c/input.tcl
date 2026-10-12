set code [catch {package require tcltest} result]
puts [list LOAD $code $result]
if {$code} {puts [list ERRORINFO $errorInfo ERRORCODE $errorCode]} else {
    puts [list HARNESS [info patchlevel] [package present tcltest] [namespace which ::tcltest::test]]
    set code [catch {
        ::tcltest::configure -verbose {}
        ::tcltest::test bootstrap-names-1 {namespace variable proc alias} -body {
            namespace eval ::startup_names {variable value ready;proc get {} {variable value;return $value}}
            interp alias {} ::startup_alias {} ::startup_names::get
            set value [::startup_alias]
            rename ::startup_alias {}
            namespace delete ::startup_names
            set value
        } -result ready
    } result]
    puts [list TEST $code $result]
    if {$code} {puts [list ERRORINFO $errorInfo ERRORCODE $errorCode]} else {puts [list COUNTERS $::tcltest::numTests(Total) $::tcltest::numTests(Passed) $::tcltest::numTests(Skipped) $::tcltest::numTests(Failed)]}
}
