proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
	set outData {}
	set errData {}
	rename ::puts [namespace current]::Replace::Puts
	namespace eval :: [list namespace import [namespace origin Replace::puts]]
	namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
	namespace forget puts
	namespace eval :: namespace forget puts
	rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}
